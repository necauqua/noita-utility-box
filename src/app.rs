use std::{sync::Arc, time::Duration};

use eframe::{
    egui::{self, ViewportBuilder},
    get_value, icon_data, set_value, NativeOptions,
};
use noita_utility_box::noita::{Noita, Seed};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumMessage, IntoEnumIterator};

use crate::{
    tools::{
        address_maps::AddressMaps, live_stats::LiveStats, material_pipette::MaterialPipette,
        orb_radar::OrbRadar, process_panel::ProcessPanel, settings::Settings,
    },
    update_check::UpdateChecker,
    util::{persist, Tickable, UpdatableApp},
};

#[derive(Default)]
pub struct AppState {
    pub current_tab: CurrentTab,

    pub settings: Settings,
    pub address_maps: AddressMaps,

    pub noita: Option<Noita>,
    pub seed: Option<Seed>,
}

persist!(AppState {
    current_tab: CurrentTab,
    settings: Settings,
    address_maps: AddressMaps,
});

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NoitaUtilityBox {
    show_process_panel: bool,

    process_panel: ProcessPanel,
    #[serde(skip)]
    orb_radar: OrbRadar,
    live_stats: LiveStats,
    material_pipette: MaterialPipette,
    state: AppState,

    #[serde(skip)]
    update_checker: UpdateChecker,

    #[cfg(debug_assertions)]
    #[serde(skip)]
    repaints: u64,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, EnumIter, EnumMessage)]
pub enum CurrentTab {
    /// Orb Radar
    #[default]
    OrbRadar,
    /// Live Stats
    LiveStats,
    /// Material Pipette
    MaterialPipette,
    /// Address Maps
    AddressMaps,
    /// Settings
    Settings,
}

impl Tickable for NoitaUtilityBox {
    fn tick(&mut self, ctx: &egui::Context) -> std::time::Duration {
        self.process_panel.update(ctx, &mut self.state);
        self.live_stats.update(ctx, &mut self.state);
        Duration::from_secs_f32(self.state.settings.background_update_interval)
    }
}

impl eframe::App for NoitaUtilityBox {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_checker.check(ctx, &mut self.state);

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.add_space(0.5);
            ui.horizontal_wrapped(|ui| {
                egui::widgets::global_theme_preference_switch(ui);
                ui.separator();
                ui.toggle_value(&mut self.show_process_panel, "Process");
                ui.separator();

                // meh
                if self.state.current_tab == CurrentTab::MaterialPipette
                    && !self.state.settings.pipette
                {
                    self.state.current_tab = CurrentTab::AddressMaps;
                }

                for tab in CurrentTab::iter() {
                    if tab == CurrentTab::MaterialPipette && !self.state.settings.pipette {
                        continue;
                    }
                    let name = tab.get_documentation().unwrap_or_default();
                    ui.selectable_value(&mut self.state.current_tab, tab, name);
                }
            });
            ui.add_space(0.5);
        });

        egui::SidePanel::left("left").show_animated(ctx, self.show_process_panel, |ui| {
            self.process_panel.ui(ui, &mut self.state)
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            use CurrentTab as T;
            match self.state.current_tab {
                T::OrbRadar => self.orb_radar.ui(ui, &mut self.state),
                T::LiveStats => self.live_stats.ui(ui),
                T::MaterialPipette => self.material_pipette.ui(ui, &mut self.state),
                T::AddressMaps => self.state.address_maps.ui(ui),
                T::Settings => self.state.settings.ui(ui),
            }

            #[cfg(debug_assertions)]
            {
                use eframe::egui::{Align, Layout, RichText};

                ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
                    self.repaints += 1;
                    ui.label(RichText::new(format!("Repaints: {}", self.repaints)).small());
                    ui.label(
                        RichText::new("⚠ Debug build ⚠")
                            .small()
                            .color(ui.visuals().warn_fg_color),
                    )
                });
            }
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        set_value(storage, eframe::APP_KEY, &self);
    }
}

impl NoitaUtilityBox {
    pub fn run() -> eframe::Result {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();

        eframe::run_native(
            "noita-utility-box",
            NativeOptions {
                viewport: ViewportBuilder {
                    title: Some("Noita Utility Box".into()),
                    icon: Some(Arc::new(
                        icon_data::from_png_bytes(include_bytes!("../res/icon.png")).unwrap(),
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
            Box::new(|cc| {
                let app: Self = cc
                    .storage
                    .as_ref()
                    .and_then(|s| get_value(*s, eframe::APP_KEY))
                    .unwrap_or_default();

                Ok(Box::new(UpdatableApp::new(app, &cc.egui_ctx)))
            }),
        )
    }
}
