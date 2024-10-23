use eframe::egui::{
    self, Checkbox, CollapsingHeader, DragValue, FontId, Grid, Label, RichText, ScrollArea,
    TextStyle, Ui,
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
    #[default(true)]
    pub check_export_name: bool,

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
            ui.label("Note: You can permanently scale the UI with Ctrl+- and Ctrl+=");

            ui.horizontal(|ui| {
                ui.label("Also, click this:");
                egui::global_theme_preference_switch(ui);
            });

            ui.separator();

            Grid::new("settings").show(ui, |ui| {
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

                ui.checkbox(&mut s.check_export_name, "Check export name")
                    .on_hover_text("When detecting noita, check that the executable export name is 'wizard_physics.exe'");
                ui.end_row();
            });

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
