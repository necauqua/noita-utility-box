use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use derive_more::derive::Debug;
use eframe::egui::{
    self, Grid, Image, Link, ScrollArea, TextFormat, TextureOptions, Ui, ViewportBuilder,
    ViewportId, Widget, text::LayoutJob,
};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use noita_engine_reader::{
    CachedTranslations, Noita, memory::MemoryStorage, types::cell_factory::CellData,
};
use smart_default::SmartDefault;

use crate::{app::AppState, util::persist, widgets::JsonWidget};

use super::{Result, Tool, ToolError};

#[derive(Debug, SmartDefault)]
pub struct MaterialList {
    #[default(true)]
    first_update: bool,
    search_text: String,
    cell_data: Vec<Arc<CellData>>,
    cached_translations: Arc<CachedTranslations>,

    #[default(SkimMatcherV2::default().ignore_case())]
    #[debug(skip)]
    matcher: SkimMatcherV2,
    filter_buf: Vec<FilteredCellData>,

    open_materials: Vec<(ViewportId, Arc<MaterialView>)>,
}
persist!(MaterialList {
    search_text: String,
});

#[derive(Debug)]
struct FilteredCellData {
    idx: String,
    ui_name_translated: String,
    name_highlights: LayoutJob,
    ui_name_highlights: LayoutJob,
    score: i64,
    data: Arc<CellData>,
}

#[derive(Debug)]
struct MaterialView {
    ui_name_translated: String,
    texture: Option<(String, Arc<[u8]>)>,
    cell_data: Arc<CellData>,
    close_request: AtomicBool,
}

impl MaterialView {
    fn new(noita: &mut Noita, entry: &FilteredCellData) -> io::Result<Self> {
        let path = entry.data.graphics.texture_file.read(noita.proc())?;
        let texture = match &*path {
            "" => None,
            p => {
                let bytes = noita.get_file(p)?;
                Some((format!("bytes://{path}"), bytes))
            }
        };

        Ok(Self {
            ui_name_translated: entry.ui_name_translated.clone(),
            texture,
            cell_data: entry.data.clone(),
            close_request: AtomicBool::new(false),
        })
    }
}

impl Widget for &MaterialView {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        ScrollArea::both().auto_shrink(false).show(ui, |ui| {
            let json = serde_json::to_value(&self.cell_data).unwrap();

            ui.label(&self.ui_name_translated);
            if let Some(texture) = &self.texture {
                ui.add(
                    Image::new(texture.clone())
                        .tint({
                            let [r, g, b, a] = self.cell_data.graphics.color.0.to_le_bytes();
                            egui::Color32::from_rgba_premultiplied(r, g, b, a)
                        })
                        .texture_options(TextureOptions::NEAREST)
                        .fit_to_original_size(4.0),
                );
            }
            ui.add(JsonWidget::new(&json));
        });
        ui.response()
    }
}

#[typetag::serde]
impl Tool for MaterialList {
    fn ui(&mut self, ui: &mut Ui, state: &mut AppState) -> Result {
        let noita = state.get_noita()?;

        let res = ui.button("Refresh materials");
        let clicked = if self.first_update {
            self.first_update = false;
            true
        } else {
            res.clicked()
        };

        if clicked {
            self.cell_data = noita.read_cell_data()?.into_iter().map(Arc::new).collect();
            if self.cell_data.is_empty() {
                return ToolError::bad_state(
                    "CellData not initialized - did you enter a world?".to_string(),
                );
            }
            self.cached_translations = Arc::new(noita.translations()?);
            self.filter_buf.reserve(self.cell_data.len());
        }

        let changed = ui
            .horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_text).changed()
            })
            .inner;

        if clicked || changed {
            self.filter_buf.clear();

            for (idx, data) in self.cell_data.iter().enumerate() {
                let name = data.name.read(noita.proc())?;
                let ui_name = data.ui_name.read(noita.proc())?;
                let ui_name_translated = ui_name
                    .strip_prefix("$")
                    .and_then(|key| self.cached_translations.translate(key, true))
                    .unwrap_or_else(|| ui_name.to_owned());

                let name_match = self.matcher.fuzzy_indices(&name, &self.search_text);
                let ui_name_match = self
                    .matcher
                    .fuzzy_indices(&ui_name_translated, &self.search_text);

                let (score, name_indices, ui_name_indices) = match (name_match, ui_name_match) {
                    (Some((a, name_indices)), Some((b, ui_name_indices))) => {
                        (a.max(b), name_indices, ui_name_indices)
                    }
                    (Some((a, name_indices)), None) => (a, name_indices, vec![]),
                    (None, Some((b, ui_name_indices))) => (b, vec![], ui_name_indices),
                    (None, None) => continue,
                };

                let name_highlights = layout_text_with_indices(ui, &name, name_indices, true);
                let ui_name_highlights =
                    layout_text_with_indices(ui, &ui_name_translated, ui_name_indices, false);

                self.filter_buf.push(FilteredCellData {
                    idx: idx.to_string(),
                    name_highlights,
                    ui_name_highlights,
                    ui_name_translated,
                    score,
                    data: data.clone(),
                });
            }
            if !self.search_text.is_empty() {
                self.filter_buf.sort_by_key(|f| -f.score);
            }
        }

        self.open_materials.retain(|(id, view)| {
            let b = ViewportBuilder::default()
                .with_title("Material")
                .with_app_id("noita-utility-box");
            ui.ctx().show_viewport_deferred(*id, b, {
                let view = view.clone();
                move |ctx, _| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.add(&*view);
                    });
                    if ctx.input(|s| s.viewport().close_requested()) {
                        view.close_request.store(true, Ordering::Relaxed);
                    }
                }
            });
            !view.close_request.load(Ordering::Relaxed)
        });

        ScrollArea::both()
            .auto_shrink(false)
            .show(ui, |ui| {
                Grid::new("all_materials")
                    .striped(true)
                    .num_columns(3)
                    .show(ui, |ui| {
                        for entry in &self.filter_buf {
                            ui.label(entry.idx.clone());

                            if ui.add(Link::new(entry.name_highlights.clone())).clicked() {
                                let id = ViewportId::from_hash_of(&entry.idx);
                                let view = MaterialView::new(noita, entry)?;
                                self.open_materials.push((id, Arc::new(view)));
                            }

                            ui.label(entry.ui_name_highlights.clone());
                            ui.end_row();
                        }
                        Ok(())
                    })
                    .inner
            })
            .inner
    }
}

fn layout_text_with_indices(ui: &Ui, text: &str, indices: Vec<usize>, quote: bool) -> LayoutJob {
    if indices.is_empty() {
        return LayoutJob::single_section(
            if quote {
                format!("\"{text}\"")
            } else {
                text.to_owned()
            },
            TextFormat::default(),
        );
    }
    let mut layout_job = LayoutJob::default();
    if quote {
        layout_job.append("\"", 0.0, TextFormat::default());
    }
    let chars = text.chars().collect::<Vec<_>>();
    let mut last_idx = 0;
    for &idx in indices.iter() {
        let part = chars[last_idx..idx].iter().collect::<String>();
        layout_job.append(&part, 0.0, TextFormat::default());

        // could join consecutive indices into a single section
        layout_job.append(
            &String::from(chars[idx]),
            0.0,
            TextFormat {
                color: ui.visuals().strong_text_color(),
                ..TextFormat::default()
            },
        );
        last_idx = idx + 1;
    }
    if let Some(last_part) = chars.get(last_idx..) {
        layout_job.append(
            &last_part.iter().collect::<String>(),
            0.0,
            TextFormat::default(),
        );
    }
    if quote {
        layout_job.append("\"", 0.0, TextFormat::default());
    }
    layout_job
}
