use std::{env, time::Instant};

use eframe::egui::{
    Checkbox, CollapsingHeader, Context, DragValue, FontId, Grid, Label, RichText, ScrollArea,
    TextStyle,
};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;
use strum::{EnumCount, EnumIter, EnumMessage, IntoEnumIterator};

use crate::{app::AppState, update_check::RELEASE_VERSION};

#[derive(Debug, Serialize, Deserialize, Clone, SmartDefault)]
pub struct Settings {
    #[default([0.5; Interval::COUNT])]
    intervals: [f32; Interval::COUNT],
    #[default(true)]
    pub check_for_updates: bool,
    #[default(true)]
    pub notify_when_outdated: bool,
    #[default(true)]
    pub check_export_name: bool,
    #[default(true)]
    pub pipette: bool,
    #[default(true)]
    pub pipette_checklist: bool,

    #[serde(skip)]
    pub newest_version: Option<String>,
}

#[derive(Debug, Clone, Copy, EnumCount, EnumIter, EnumMessage)]
#[repr(usize)]
pub enum Interval {
    /// Noita process search interval
    NoitaSearch,
    /// Live stats polling
    LiveStats,
}

#[derive(Debug)]
pub struct Timer {
    interval: Interval,
    last_tick: Instant,
}

impl Timer {
    pub fn new(interval: Interval) -> Self {
        Self {
            interval,
            last_tick: Instant::now(),
        }
    }

    pub fn interval(&self, state: &AppState) -> f32 {
        state.settings.intervals[self.interval as usize]
    }

    pub fn go(&mut self, ctx: &Context, state: &AppState) -> bool {
        let interval = self.interval(state);
        ctx.request_repaint_after_secs(interval);
        if self.last_tick.elapsed().as_secs_f32() >= interval {
            self.last_tick = Instant::now();
            return true;
        }
        false
    }
}

impl Settings {
    pub fn ui(&mut self, ui: &mut eframe::egui::Ui) {
        ui.heading("Settings");

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            let repo = "https://github.com/necauqua/noita-utility-box";
            let (label, url) = match RELEASE_VERSION {
                Some(version) => {
                    ui.add(Label::new(RichText::new("Version: ").small()).selectable(false));
                    (version, format!("{repo}/releases/tag/v{version}"))
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

            if let Some(ref s) = self.newest_version {
                let text = RichText::new(format!(" (latest: {s})"))
                    .small()
                    .color(ui.style().visuals.weak_text_color());
                ui.add(Label::new(text).selectable(false));
            }
        });

        ui.separator();

        ScrollArea::vertical().show(ui, |ui| {
            ui.label("Note: You can permanently scale the UI with Ctrl+- and Ctrl+=");

            ui.separator();

            Grid::new("settings").show(ui, |ui| {
                for interval in Interval::iter() {
                    let Some(doc) = interval.get_documentation() else {
                        continue;
                    };
                    ui.label(doc);
                    ui.add(
                        DragValue::new(&mut self.intervals[interval as usize])
                            .range(0.0..=60.0)
                            .speed(0.02)
                            .suffix(" s"),
                    );
                    ui.end_row();
                }

                ui.checkbox(&mut self.check_for_updates, "Check for updates on startup")
                    .on_hover_text("This makes one request to the GitHub API on startup to check the latest release version");
                ui.end_row();

                if !self.check_for_updates {
                    self.notify_when_outdated = false;
                }
                ui.vertical(|ui| {
                    ui.indent("update-check", |ui| {
                        ui.add_enabled(self.check_for_updates, Checkbox::new(&mut self.notify_when_outdated, "Startup update notification"))
                            .on_hover_text("This controls the popup shown on startup if the latest release version is newer than the current version");
                    });
                });
                ui.end_row();

                ui.checkbox(&mut self.check_export_name, "Check export name")
                    .on_hover_text("When detecting noita, check that the executable export name is 'wizard_physics.exe'");
                ui.end_row();

                ui.checkbox(&mut self.pipette, "Enable Material Pipette")
                    .on_hover_text("The pipette is obviously very targeted at Fury and most people wont need it");
                ui.end_row();

                if !self.pipette {
                    self.pipette_checklist = false;
                }
                ui.vertical(|ui| {
                    ui.indent("pipette", |ui| {
                        ui.add_enabled(self.pipette, Checkbox::new(&mut self.pipette_checklist, "Add all materials checklist"))
                            .on_hover_text("Adds a checklist of all materials to the material pipette");
                    });
                });
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
