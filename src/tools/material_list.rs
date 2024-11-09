use std::sync::Arc;

use eframe::egui::{text::LayoutJob, Grid, ScrollArea, TextFormat, Ui};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use noita_utility_box::{
    memory::MemoryStorage,
    noita::{types::cell_factory::CellData, CachedTranslations},
};

use crate::{app::AppState, util::persist};

use super::{Result, Tool, ToolError};

#[derive(Debug, Default)]
pub struct MaterialList {
    search_text: String,
    cell_data: Vec<Arc<CellData>>,
    cached_translations: CachedTranslations,

    filter_buf: Vec<FilteredCellData>,
}
persist!(MaterialList {
    search_text: String,
});

#[derive(Debug)]
struct FilteredCellData {
    idx: String,
    name: LayoutJob,
    ui_name: LayoutJob,
    score: i64,
    _data: Arc<CellData>,
}

#[typetag::serde]
impl Tool for MaterialList {
    fn ui(&mut self, ui: &mut Ui, state: &mut AppState) -> Result {
        let Some(noita) = state.noita.as_mut() else {
            ui.label("Noita not connected");
            return Ok(());
        };

        let text = match self.cell_data.is_empty() {
            true => "Read materials",
            false => "Refresh materials",
        };

        let clicked = ui.button(text).clicked();
        if clicked {
            self.cell_data = noita.read_cell_data()?.into_iter().map(Arc::new).collect();
            if self.cell_data.is_empty() {
                return ToolError::bad_state(
                    "CellData not initialized - did you enter a world?".to_string(),
                );
            }
            self.cached_translations = noita.translations()?;
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

            if self.search_text.is_empty() {
                for (idx, data) in self.cell_data.iter().enumerate() {
                    let name = data.name.read(noita.proc())?;
                    let ui_name = data.ui_name.read(noita.proc())?;
                    let ui_name = ui_name
                        .strip_prefix("$")
                        .map(|key| self.cached_translations.translate(key, true).into_owned())
                        .unwrap_or(ui_name);

                    self.filter_buf.push(FilteredCellData {
                        idx: idx.to_string(),
                        name: layout_text_with_indices(ui, name, vec![], true),
                        ui_name: layout_text_with_indices(ui, ui_name, vec![], false),
                        score: 0,
                        _data: data.clone(),
                    });
                }
            } else {
                let matcher = SkimMatcherV2::default().ignore_case();
                for (idx, data) in self.cell_data.iter().enumerate() {
                    let name = data.name.read(noita.proc())?;
                    let ui_name = data.ui_name.read(noita.proc())?;

                    let ui_name = ui_name
                        .strip_prefix("$")
                        .map(|key| self.cached_translations.translate(key, true).into_owned())
                        .unwrap_or(ui_name);

                    let name_match = matcher.fuzzy_indices(&name, &self.search_text);
                    let ui_name_match = matcher.fuzzy_indices(&ui_name, &self.search_text);

                    let (score, name_indices, ui_name_indices) = match (name_match, ui_name_match) {
                        (Some((a, name_indices)), Some((b, ui_name_indices))) => {
                            (a.max(b), name_indices, ui_name_indices)
                        }
                        (Some((a, name_indices)), None) => (a, name_indices, vec![]),
                        (None, Some((b, ui_name_indices))) => (b, vec![], ui_name_indices),
                        (None, None) => continue,
                    };

                    let name = layout_text_with_indices(ui, name, name_indices, true);
                    let ui_name = layout_text_with_indices(ui, ui_name, ui_name_indices, false);

                    self.filter_buf.push(FilteredCellData {
                        idx: idx.to_string(),
                        name,
                        ui_name,
                        score,
                        _data: data.clone(),
                    });
                }
                self.filter_buf.sort_by_key(|f| -f.score);
            }
        }

        ScrollArea::both()
            .show(ui, |ui| {
                Grid::new("all_materials")
                    .num_columns(3)
                    .show(ui, |ui| {
                        for entry in &self.filter_buf {
                            ui.label(entry.idx.clone());
                            ui.label(entry.name.clone());
                            ui.label(entry.ui_name.clone());
                            ui.end_row();
                        }
                        Ok(())
                    })
                    .inner
            })
            .inner
    }
}

fn layout_text_with_indices(ui: &Ui, text: String, indices: Vec<usize>, quote: bool) -> LayoutJob {
    if indices.is_empty() {
        return LayoutJob::single_section(
            if quote { format!("\"{text}\"") } else { text },
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
