use std::{collections::HashMap, sync::Arc};

use anyhow::Context as _;
use anyhow::bail;
use eframe::egui::{ComboBox, Context, DragValue, Grid, RichText, TextEdit, Ui};
use futures::{StreamExt, pin_mut};
use noita_engine_reader::memory::MemoryStorage;
use obws::{events::Event, requests::inputs::SetSettings, responses::inputs::InputId};
use smart_default::SmartDefault;
use strfmt::{FmtError, Format};

use crate::{
    app::AppState,
    util::{Promise, persist},
};
use derive_more::Debug;

use super::{Result, Tool};

#[derive(Debug, Default)]
enum ObsState {
    #[default]
    NotConnected,
    Connecting(#[debug(skip)] Promise<obws::error::Result<obws::Client>>),
    Connected(#[debug(skip)] Arc<obws::Client>, Promise<()>),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Stats {
    deaths: u32,
    wins: u32,
    streak: u32,
    record: u32,
    actual_playtime: String,
}

#[derive(Debug, SmartDefault)]
pub struct LiveStats {
    stats: Option<std::result::Result<Stats, String>>,

    obs_ws: ObsState,
    text_sources: Promise<Vec<InputId>>,

    format_error: Option<String>,
    /// A signal to force an update of the OBS text source
    format_changed: bool,

    #[default("localhost")]
    obs_address: String,
    #[default(4455)]
    obs_port: u16,
    obs_password: String,
    selected: Option<InputId>,
    #[default = "{deaths}/{wins}/{streak}({streak-pb})"]
    format: String,

    /// Used for persistence
    was_connected: bool,
}

persist!(LiveStats {
   obs_address: String,
   obs_port: u16,
   obs_password: String,
   selected: Option<InputId>,
   format: String,
   was_connected: bool,
});

impl LiveStats {
    fn connect(&mut self) {
        self.obs_ws = ObsState::Connecting(Promise::spawn(obws::Client::connect(
            self.obs_address.clone(),
            self.obs_port,
            Some(self.obs_password.clone()),
        )));
    }

    fn disconnect(&mut self) {
        self.obs_ws = ObsState::NotConnected;
        self.was_connected = false;
    }
}

#[typetag::serde]
impl Tool for LiveStats {
    fn tick(&mut self, ctx: &Context, state: &mut AppState) {
        let Some(noita) = &state.noita else {
            return;
        };

        let new_stats = noita
            .read_stats()
            .context("Reading global stats")
            .and_then(|global| {
                if global.key_value_stats.is_empty() {
                    bail!("key_value_stats is empty");
                }

                let end0 = global
                    .key_value_stats
                    .get(noita.proc(), "progress_ending0")
                    .context("Getting progress_ending0 stat")?
                    .unwrap_or_default();
                let end1 = global
                    .key_value_stats
                    .get(noita.proc(), "progress_ending1")
                    .context("Getting progress_ending1 stat")?
                    .unwrap_or_default();

                anyhow::Ok(Stats {
                    deaths: global.global.death_count,
                    wins: end0 + end1,
                    streak: global.session.streaks,
                    record: global.highest.streaks,
                    actual_playtime: global.global.playtime_str.read(noita.proc())?,
                })
            })
            .map_err(|e| format!("{e:#}"));

        if self.stats.as_ref().is_some_and(|r| *r == new_stats) && !self.format_changed {
            return;
        }

        // wake up the ui to redraw the stats
        ctx.request_repaint();

        self.format_changed = false;
        self.stats = Some(new_stats);

        if let (Some(Ok(stats)), Some(selected), ObsState::Connected(client, _)) =
            (&self.stats, &self.selected, &self.obs_ws)
        {
            let data = HashMap::from([
                ("deaths".to_owned(), stats.deaths),
                ("wins".to_owned(), stats.wins),
                ("streak".to_owned(), stats.streak),
                ("streak-pb".to_owned(), stats.record),
            ]);

            let formatted = match self.format.format(&data) {
                Err(
                    FmtError::Invalid(msg) | FmtError::KeyError(msg) | FmtError::TypeError(msg),
                ) => {
                    self.format_error = Some(format!("Bad format: {msg}"));
                    return;
                }
                Ok(f) => f,
            };

            let src = selected.clone();
            let client = client.clone();
            tokio::spawn(async move {
                tracing::info!(
                    src.name,
                    src.uuid = src.uuid.to_string(),
                    text = formatted,
                    "updating OBS text source"
                );
                let params = SetSettings {
                    input: (&src).into(),
                    settings: &HashMap::from([("text", formatted)]),
                    overlay: None,
                };
                if let Err(e) = client.inputs().set_settings(params).await {
                    tracing::error!(
                        src.name,
                        src.uuid = src.uuid.to_string(),
                        "failed to update OBS text source: {e:#}",
                    );
                }
            });
        }
    }

    fn ui(&mut self, ui: &mut Ui, _state: &mut AppState) -> Result {
        match &self.stats {
            Some(Ok(s)) => {
                Grid::new("live_stats").show(ui, |ui| {
                    ui.label("Deaths: ");
                    ui.label(s.deaths.to_string());
                    ui.end_row();

                    ui.label("Wins: ");
                    ui.label(s.wins.to_string());
                    ui.end_row();

                    ui.label("Streak: ");
                    ui.label(s.streak.to_string());
                    ui.end_row();

                    ui.label("Record: ");
                    ui.label(s.record.to_string());
                    ui.end_row();
                });

                ui.label(format!(
                    "Your actual playtime (without AFK and pausing) as recorded by Noita is: {}",
                    s.actual_playtime
                ));
                ui.end_row();
            }
            Some(Err(e)) => {
                ui.label(RichText::new(e).color(ui.style().visuals.error_fg_color));
                ui.end_row();
            }
            None => {
                ui.label("No data");
                ui.end_row();
            }
        }

        ui.separator();

        ui.label("Format:");
        if ui.add(TextEdit::multiline(&mut self.format)).changed() {
            self.format_error = None;
            self.format_changed = true;
        }
        if let Some(format_error) = &self.format_error {
            ui.label(RichText::new(format_error).color(ui.style().visuals.error_fg_color));
        }

        ui.separator();

        match &mut self.obs_ws {
            ObsState::NotConnected => {
                ui.label("Connect to OBS");

                Grid::new("obs_connect").show(ui, |ui| {
                    ui.label("Address:");

                    ui.horizontal(|ui| {
                        ui.style_mut().spacing.item_spacing = [2.0, 0.0].into();
                        ui.add(
                            TextEdit::singleline(&mut self.obs_address), // .min_size([ui.available_width(), 20.0].into()),
                        );

                        ui.add(DragValue::new(&mut self.obs_port));
                    });
                    ui.end_row();

                    ui.label("Password:");
                    ui.add(TextEdit::singleline(&mut self.obs_password).password(true));
                    ui.end_row();
                });
                if ui.button("Connect").clicked() || self.was_connected {
                    self.connect();
                }
            }
            ObsState::Connecting(p) => match p.poll_take() {
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Connecting to OBS...");
                    });
                }
                Some(Err(e)) => {
                    self.obs_ws = ObsState::Error(format!("{e:#}"));
                }
                Some(Ok(client)) => {
                    self.obs_ws = match client.events() {
                        Ok(events) => {
                            let ctx = ui.ctx().clone();
                            let end_promise = Promise::spawn(async move {
                                pin_mut!(events);
                                while let Some(event) = events.next().await {
                                    if let Event::ServerStopping = event {
                                        ctx.request_repaint();
                                        break;
                                    }
                                }
                            });
                            self.was_connected = true;
                            ObsState::Connected(Arc::new(client), end_promise)
                        }
                        Err(e) => ObsState::Error(format!("{e:#}")),
                    }
                }
            },
            ObsState::Connected(client, end_promise) => {
                if end_promise.poll().is_some() {
                    self.disconnect();
                    return Ok(());
                }
                // stop referencing self.obs_ws via this client through the big match
                let client = (*client).clone();

                Grid::new("obs_connected").show(ui, |ui| {
                    ui.label("Connected to OBS");
                    if ui.button("Disconnect").clicked() {
                        self.disconnect();
                    }
                    ui.end_row();

                    ui.label("Select text source");
                    let r = ComboBox::from_id_salt("obs_text_source")
                        .selected_text(self.selected.as_ref().map_or("", |id| &id.name))
                        .show_ui(ui, |ui| {
                            for source in self.text_sources.poll_or_default::<[_]>() {
                                ui.selectable_value(
                                    &mut self.selected,
                                    Some(source.clone()),
                                    &source.name,
                                );
                            }
                        });
                    if r.response.clicked() {
                        let client = client.clone();
                        self.text_sources = Promise::spawn(async move {
                            client
                                .inputs()
                                .list(Some("text_ft2_source_v2"))
                                .await
                                .map(|inputs| inputs.into_iter().map(|input| input.id).collect())
                                .unwrap_or_default()
                        });
                    }

                    ui.end_row();
                });
            }
            ObsState::Error(e) => {
                ui.label(
                    RichText::new(format!("OBS error: {e}"))
                        .color(ui.style().visuals.error_fg_color),
                );
                ui.horizontal(|ui| {
                    if ui.button("Retry").clicked() {
                        self.connect();
                    }
                    if ui.button("Cancel").clicked() {
                        self.disconnect();
                    }
                });
            }
        }
        Ok(())
    }
}
