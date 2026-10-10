use std::collections::HashMap;

use anyhow::Context as _;
use eframe::egui::{Align, Context, Layout, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use noita_engine_reader::{
    Noita,
    memory::{MemoryStorage, Ptr},
    types::profiler::{PROFILER_HISTORY, ProfilerData},
};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

use crate::{app::AppState, util::persist};

use super::{Result, Tool, ToolError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
enum SortBy {
    Name,
    #[default]
    Average,
    Peak,
    Last,
    Min,
    Max,
    Samples,
    Total,
}

/// The columns of the table: what they sort by, their title and their tooltip.
const COLUMNS: &[(SortBy, &str, &str)] = &[
    (
        SortBy::Name,
        "Component",
        "The component that the system updates",
    ),
    (
        SortBy::Average,
        "Avg ms/frame",
        "The milliseconds per game frame since the previous update of this tool \
         (see the background update interval in the settings)",
    ),
    (
        SortBy::Peak,
        "Peak ms",
        "The largest of the last 30 samples",
    ),
    (SortBy::Last, "Last ms", "The last sample"),
    (
        SortBy::Min,
        "Min ms",
        "The smallest sample since Noita started",
    ),
    (
        SortBy::Max,
        "Max ms",
        "The largest sample since Noita started",
    ),
    (
        SortBy::Samples,
        "Samples",
        "The number of samples since Noita started; a system adds one sample in \
         each frame in which it runs",
    ),
    (
        SortBy::Total,
        "Total ms",
        "The sum of all the samples since Noita started",
    ),
];

#[derive(Debug)]
struct Row {
    name: String,
    /// The milliseconds per frame, if there are averages.
    avg_ms: Option<f64>,
    /// The largest of the last [`PROFILER_HISTORY`] samples.
    peak_ms: f64,
    data: ProfilerData,
}

impl Row {
    fn key(&self, sort_by: SortBy) -> f64 {
        match sort_by {
            SortBy::Name => 0.0,
            SortBy::Average => self.avg_ms.unwrap_or(-1.0),
            SortBy::Peak => self.peak_ms,
            SortBy::Last => self.data.last_ms,
            SortBy::Min => self.data.min_ms,
            SortBy::Max => self.data.max_ms,
            SortBy::Samples => self.data.count as f64,
            SortBy::Total => self.data.total_ms,
        }
    }
}

/// The totals of all the scopes at a frame.
#[derive(Debug)]
struct Snapshot {
    frame: u32,
    totals: HashMap<String, f64>,
}

/// Shows the time that each component system takes, from the profiler that is
/// built into the game.
#[derive(Debug, SmartDefault)]
pub struct ComponentProfiler {
    /// The scopes of the profiler; read again when their number changes.
    scopes: Vec<(String, Ptr<ProfilerData>)>,
    /// The totals that the averages start from.
    previous: Option<Snapshot>,
    /// The milliseconds per frame of each scope, from `previous` to the frame
    /// before it advanced.
    averages: HashMap<String, f64>,
    /// The number of frames that `averages` are over.
    frames: u32,
    #[default(Ok(Vec::new()))]
    rows: std::result::Result<Vec<Row>, String>,
    /// Whether the tool was on the screen since the previous tick.
    shown: bool,

    sort_by: SortBy,
    #[default(true)]
    descending: bool,
}

persist!(ComponentProfiler {
    sort_by: SortBy,
    descending: bool,
});

/// `class LuaComponent` -> `LuaComponent`.
fn display_name(name: &str) -> &str {
    name.strip_prefix("class ").unwrap_or(name)
}

impl ComponentProfiler {
    fn forget(&mut self) {
        self.previous = None;
        self.averages.clear();
        self.frames = 0;
    }

    fn sample(&mut self, noita: &Noita) -> anyhow::Result<()> {
        let proc = noita.proc();

        let Some(profiler) = noita
            .read_simple_profiler()
            .context("Reading the profiler")?
        else {
            self.forget();
            self.rows = Ok(Vec::new());
            return Ok(());
        };
        if profiler.data.len() as usize != self.scopes.len() {
            self.scopes = profiler
                .data
                .read(proc)
                .context("Reading the scopes of the profiler")?
                .into_iter()
                .collect();
        }

        let frame = noita
            .read_game_global()
            .context("Reading the frame number")?
            .frame_counter;

        let mut scopes = Vec::with_capacity(self.scopes.len());
        for (name, ptr) in &self.scopes {
            let data = ptr
                .read(proc)
                .with_context(|| format!("Reading the profiler data of {name}"))?;
            scopes.push((name, data));
        }

        match &self.previous {
            // while the game is paused, the averages stay
            Some(previous) if previous.frame == frame => {}
            Some(previous) if previous.frame < frame => {
                let frames = frame - previous.frame;
                self.averages = scopes
                    .iter()
                    .map(|(name, data)| {
                        // a scope that is new since the previous snapshot has all
                        // of its time since then
                        let before = previous.totals.get(*name).copied().unwrap_or(0.0);
                        let avg = (data.total_ms - before).max(0.0) / frames as f64;
                        ((*name).clone(), avg)
                    })
                    .collect();
                self.frames = frames;
                self.previous = None;
            }
            // nothing to compare with yet, or the frame number went back
            // in a new game
            _ => {
                self.previous = None;
                self.averages.clear();
                self.frames = 0;
            }
        }
        if self.previous.is_none() {
            self.previous = Some(Snapshot {
                frame,
                totals: scopes
                    .iter()
                    .map(|(name, data)| ((*name).clone(), data.total_ms))
                    .collect(),
            });
        }

        self.rows = Ok(scopes
            .into_iter()
            .map(|(name, data)| Row {
                name: name.clone(),
                avg_ms: self.averages.get(name).copied(),
                peak_ms: data.history_ms.iter().copied().fold(0.0, f64::max),
                data,
            })
            .collect());
        Ok(())
    }

    fn header_clicked(&mut self, column: SortBy) {
        if self.sort_by == column {
            self.descending = !self.descending;
        } else {
            self.sort_by = column;
            self.descending = column != SortBy::Name;
        }
    }
}

fn sort(rows: &mut [Row], sort_by: SortBy, descending: bool) {
    rows.sort_by(|a, b| {
        let order = match sort_by {
            SortBy::Name => display_name(&a.name).cmp(display_name(&b.name)),
            _ => a.key(sort_by).total_cmp(&b.key(sort_by)),
        };
        let order = if descending { order.reverse() } else { order };
        // a stable order for the same keys
        order.then_with(|| a.name.cmp(&b.name))
    });
}

fn number(ui: &mut Ui, text: String) {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.label(RichText::new(text).monospace());
    });
}

#[typetag::serde]
impl Tool for ComponentProfiler {
    fn tick(&mut self, ctx: &Context, state: &mut AppState) {
        // read only while the tool is on the screen
        if !std::mem::take(&mut self.shown) {
            self.forget();
            return;
        }
        let Some(noita) = &state.noita else {
            self.forget();
            return;
        };
        if let Err(e) = self.sample(noita) {
            self.forget();
            self.rows = Err(format!("{e:#}"));
        }
        ctx.request_repaint();
    }

    fn ui(&mut self, ui: &mut Ui, state: &mut AppState) -> Result {
        self.shown = true;
        state.get_noita()?;

        let rows = match &mut self.rows {
            Ok(rows) if rows.is_empty() => {
                return ToolError::retry("The game did not run any component systems yet");
            }
            Ok(rows) => rows,
            Err(e) => return ToolError::retry(e.clone()),
        };
        sort(rows, self.sort_by, self.descending);
        let rows = &*rows;

        ui.label(
            RichText::new(format!(
                "The time that each component system takes, from the profiler that is built \
                 into Noita. A sample is the time of one update of a system; Peak is the \
                 largest of the last {PROFILER_HISTORY} samples."
            ))
            .weak(),
        );
        if self.frames == 0 {
            ui.label("Avg ms/frame: waiting for the game to run frames");
        } else {
            let total: f64 = rows.iter().filter_map(|r| r.avg_ms).sum();
            ui.label(format!(
                "All systems: {total:.3} ms/frame on average over the last {} frames",
                self.frames
            ));
        }
        ui.separator();

        let row_height = ui.spacing().interact_size.y;
        let mut clicked = None;
        TableBuilder::new(ui)
            .id_salt("component_profiler")
            .striped(true)
            .resizable(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::auto().at_least(120.0).clip(true))
            .columns(Column::auto().at_least(50.0), COLUMNS.len() - 2)
            .column(Column::remainder().at_least(50.0))
            .header(row_height, |mut header| {
                for &(column, title, hover) in COLUMNS {
                    header.col(|ui| {
                        let title = if self.sort_by != column {
                            title.to_owned()
                        } else if self.descending {
                            format!("{title} ⏷")
                        } else {
                            format!("{title} ⏶")
                        };
                        let response = ui
                            .selectable_label(self.sort_by == column, RichText::new(title).strong())
                            .on_hover_text(hover);
                        if response.clicked() {
                            clicked = Some(column);
                        }
                    });
                }
            })
            .body(|body| {
                body.rows(row_height, rows.len(), |mut table_row| {
                    let row = &rows[table_row.index()];
                    table_row.col(|ui| {
                        ui.label(display_name(&row.name)).on_hover_text(&row.name);
                    });
                    table_row.col(|ui| {
                        number(ui, row.avg_ms.map_or("-".into(), |avg| format!("{avg:.3}")));
                    });
                    table_row.col(|ui| number(ui, format!("{:.3}", row.peak_ms)));
                    table_row.col(|ui| number(ui, format!("{:.3}", row.data.last_ms)));
                    table_row.col(|ui| number(ui, format!("{:.3}", row.data.min_ms)));
                    table_row.col(|ui| number(ui, format!("{:.3}", row.data.max_ms)));
                    table_row.col(|ui| number(ui, row.data.count.to_string()));
                    table_row.col(|ui| number(ui, format!("{:.1}", row.data.total_ms)));
                });
            });
        if let Some(column) = clicked {
            self.header_clicked(column);
        }
        Ok(())
    }
}
