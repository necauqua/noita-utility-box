use eframe::egui::{
    self, Checkbox, CollapsingHeader, Color32, DragValue, FontId, Grid, Label, RichText,
    ScrollArea, TextStyle, Ui,
};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

use crate::{app::AppState, update_check::RELEASE_VERSION};

use super::{Result, Tool};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings;

#[derive(Debug, Serialize, Deserialize, Clone, SmartDefault)]
#[serde(default)]
pub struct SettingsData {
    #[default(0.5)]
    pub background_update_interval: f32,
    #[default(true)]
    pub check_for_updates: bool,
    #[default(true)]
    pub notify_when_outdated: bool,

    #[default(Color32::GOLD)]
    pub color_orb_chests: Color32,
    #[default(Color32::BLUE)]
    pub color_orb_rooms: Color32,

    #[serde(skip)]
    pub newest_version: Option<String>,
}

#[typetag::serde]
impl Tool for Settings {
    fn ui(&mut self, ui: &mut Ui, state: &mut AppState) -> Result {
        self.ui(ui, state);
        Ok(())
    }
}

impl Settings {
    pub fn ui(&mut self, ui: &mut Ui, state: &mut AppState) {
        let s = &mut state.settings;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            let repo = "https://github.com/necauqua/noita-utility-box";
            let (label, url) = match RELEASE_VERSION {
                Some(version) => {
                    ui.add(Label::new(RichText::new("Version: ").small()).selectable(false));
                    (version, format!("{repo}/releases/tag/{version}"))
                }
                None => {
                    ui.add(Label::new(RichText::new("Build: ").small()).selectable(false));
                    let commit = env!("BUILD_COMMIT");
                    (env!("BUILD_INFO"), format!("{repo}/tree/{commit}"))
                }
            };

            let font = FontId::monospace(TextStyle::Small.resolve(ui.style()).size);
            let info = RichText::new(label).font(font);
            ui.hyperlink_to(info, url.clone()).on_hover_text(url);

            if let Some(s) = &s.newest_version {
                let text = RichText::new(format!(" (latest: {s})"))
                    .small()
                    .color(ui.style().visuals.weak_text_color());
                ui.add(Label::new(text).selectable(false));
            }
        });

        ui.separator();

        ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Settings");
            ui.end_row();
            ui.label("Note: You can permanently scale the UI with Ctrl+- and Ctrl+=");

            ui.horizontal(|ui| {
                ui.label("Also, click this:");
                egui::global_theme_preference_switch(ui);
            });

            ui.separator();

            Grid::new("general_settings").show(ui, |ui| {
                ui.heading("General Settings");
                ui.end_row();

                ui.label("Background updates interval")
                    .on_hover_text("How often the background updates run (used by live stats and noita process auto-detection)");
                ui.add(
                    DragValue::new(&mut s.background_update_interval)
                        .range(0.0..=60.0)
                        .speed(0.02)
                        .suffix(" s"),
                );
                ui.end_row();

                if RELEASE_VERSION.is_some() {
                    ui.checkbox(&mut s.check_for_updates, "Check for updates on startup")
                        .on_hover_text("This makes one request to the GitHub API on startup to check the latest release version");
                    ui.end_row();

                    if !s.check_for_updates {
                        s.notify_when_outdated = false;
                    }
                    ui.vertical(|ui| {
                        ui.indent("update-check", |ui| {
                            ui.add_enabled(s.check_for_updates, Checkbox::new(&mut s.notify_when_outdated, "Startup update notification"))
                                .on_hover_text("This controls the popup shown on startup if the latest release version is newer than the current version");
                        });
                    });
                    ui.end_row();
                }

                ui.end_row();
            });

            ui.separator();

            Grid::new("radar_settings").show(ui, |ui| {
                ui.heading("Radar Settings");
                ui.end_row();

                ui.label("Greater Chest Orbs color:")
                    .on_hover_text("Only apply if 'Show orb rooms' is enabled");
                ui.color_edit_button_srgba(&mut s.color_orb_chests);
                ui.end_row();

                ui.label("Orb rooms color:");
                ui.color_edit_button_srgba(&mut s.color_orb_rooms);
                ui.end_row();
            });

            ui.separator();

            CollapsingHeader::new("egui").show(ui, |ui| {
                let prev_options = ui.ctx().options(|o| o.clone());
                let mut options = prev_options.clone();

                options.ui(ui);

                if options != prev_options {
                    ui.ctx().options_mut(move |o| *o = options);
                }
            });
        });
    }
}
