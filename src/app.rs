use std::{sync::Arc, time::Duration};

use eframe::{
    egui::{self, Frame, TextWrapMode, Ui, ViewportBuilder, WidgetText},
    get_value, icon_data, set_value, NativeOptions,
};
use egui_tiles::{Container, Linear, LinearDir, SimplificationOptions, Tabs, Tile, TileId, Tiles};
use noita_utility_box::noita::{Noita, Seed};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

use crate::{
    tools::{address_maps::AddressMapsData, settings::SettingsData, Tool, TOOLS},
    update_check::UpdateChecker,
    util::{persist, Tickable, UpdatableApp},
};

#[derive(Default)]
pub struct AppState {
    pub settings: SettingsData,
    pub address_maps: AddressMapsData,

    hidden_tools: Vec<Pane>,
    tool_request: Option<(TileId, Pane)>,

    pub noita: Option<Noita>,
    pub seed: Option<Seed>,

    #[cfg(debug_assertions)]
    repaints: u64,
}

persist!(AppState {
    settings: SettingsData,
    address_maps: AddressMapsData,
    hidden_tools: Vec<Pane>,
});

#[derive(Serialize, Deserialize, SmartDefault)]
#[serde(default)]
pub struct NoitaUtilityBox {
    state: AppState,

    #[serde(skip)]
    update_checker: UpdateChecker,

    #[default(default_tree())]
    tree: egui_tiles::Tree<Pane>,
}

#[derive(Serialize, Deserialize)]
struct Pane {
    title: String,
    tool: Box<dyn Tool>,
}

fn default_tree() -> egui_tiles::Tree<Pane> {
    let mut tiles = egui_tiles::Tiles::default();

    // first tool is the process panel
    let [first, rest @ ..] = TOOLS else {
        panic!("No tools defined");
    };

    let split_tab = vec![tiles.insert_pane(Pane {
        title: first.title.into(),
        tool: (first.default_constructor)(),
    })];

    let mut tabs = vec![];
    for tool in rest {
        tabs.push(tiles.insert_pane(Pane {
            title: tool.title.into(),
            tool: (tool.default_constructor)(),
        }));
    }

    let split_tab = tiles.insert_tab_tile(split_tab);
    let tabs = tiles.insert_tab_tile(tabs);

    // make it 0.3|0.7
    let root = tiles.insert_new(Tile::Container(Container::Linear(Linear::new_binary(
        LinearDir::Horizontal,
        [split_tab, tabs],
        0.3,
    ))));

    egui_tiles::Tree::new("tool_tree", root, tiles)
}

impl egui_tiles::Behavior<Pane> for AppState {
    fn simplification_options(&self) -> SimplificationOptions {
        SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
    fn tab_title_for_pane(&mut self, pane: &Pane) -> WidgetText {
        pane.title.clone().into()
    }

    fn on_tab_close(&mut self, tiles: &mut Tiles<Pane>, tile_id: TileId) -> bool {
        if let Some(Tile::Pane(pane)) = tiles.remove(tile_id) {
            self.hidden_tools.push(pane);
        }
        false // we removed it ourselves (to get ownership)
    }

    fn is_tab_closable(&self, tiles: &Tiles<Pane>, _tile_id: TileId) -> bool {
        // disallow closing the last tab
        let mut iter = tiles.tiles();
        iter.next().is_some() && iter.next().is_some()
    }

    fn top_bar_right_ui(
        &mut self,
        _tiles: &Tiles<Pane>,
        ui: &mut Ui,
        tile_id: TileId,
        _tabs: &Tabs,
        _scroll_offset: &mut f32,
    ) {
        if self.hidden_tools.is_empty() {
            return;
        }
        egui::menu::menu_button(ui, "➕", |ui| {
            ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
            let mut clicked = None;
            for (i, closed) in self.hidden_tools.iter().enumerate() {
                if ui.button(&closed.title).clicked() {
                    clicked = Some(i);
                }
            }

            // postpone re-adding the tool until after the tree finishes drawing,
            // where we actually have the (mutable) reference to the tree
            self.tool_request = clicked.map(|i| (tile_id, self.hidden_tools.remove(i)));
        });
        ui.add_space(4.0);
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        // re-add margins but inside of the panes
        Frame::central_panel(ui.style()).show(ui, |ui| {
            pane.tool.ui(ui, self);

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

        egui_tiles::UiResponse::None
    }
}

impl Tickable for NoitaUtilityBox {
    fn tick(&mut self, ctx: &egui::Context) -> std::time::Duration {
        for tile in self.tree.tiles.tiles_mut() {
            if let Tile::Pane(pane) = tile {
                pane.tool.tick(ctx, &mut self.state);
            }
        }

        // untie the &mut hidden tools from &mut state
        let mut hidden_tools = std::mem::take(&mut self.state.hidden_tools);
        for tile in &mut hidden_tools {
            tile.tool.tick(ctx, &mut self.state);
        }
        self.state.hidden_tools = hidden_tools;

        Duration::from_secs_f32(self.state.settings.background_update_interval)
    }
}

impl eframe::App for NoitaUtilityBox {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_checker.check(ctx, &mut self.state);

        egui::CentralPanel::default()
            // remove margin
            .frame(Frame::none().fill(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| {
                self.tree.ui(&mut self.state, ui);

                if let Some((tile_id, tool)) = self.state.tool_request.take() {
                    let pane = self.tree.tiles.insert_pane(tool);
                    self.tree
                        .move_tile_to_container(pane, tile_id, usize::MAX, true);
                }
            });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        set_value(storage, eframe::APP_KEY, &self);
    }
}

impl NoitaUtilityBox {
    fn ensure_all_tools_present(&mut self) {
        // TODO
        // Since we load panes from storage, check that all tools are added to the state
        // Otherwise a tool potentially can be lost forever somehow
        // (unless the user knows to fully delete state so the default kicks in)
        //
        // this would require dyn Tool <-> ToolInfo equality that's not the title though, meh
    }

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
                let mut app: Self = cc
                    .storage
                    .as_ref()
                    .and_then(|s| get_value(*s, eframe::APP_KEY))
                    .unwrap_or_default();

                app.ensure_all_tools_present();

                Ok(Box::new(UpdatableApp::new(app, &cc.egui_ctx)))
            }),
        )
    }
}
