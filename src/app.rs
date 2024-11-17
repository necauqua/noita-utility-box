use std::{collections::HashSet, sync::Arc, time::Duration};

use eframe::{
    egui::{self, Frame, RichText, TextWrapMode, Ui, ViewportBuilder, WidgetText},
    get_value, icon_data, set_value, NativeOptions,
};
use egui_tiles::{Container, Linear, LinearDir, SimplificationOptions, Tabs, Tile, TileId, Tiles};
use noita_utility_box::noita::{Noita, Seed};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

use crate::{
    tools::{
        address_maps::AddressMapsData, settings::SettingsData, Tool, ToolError, ToolInfo, TOOLS,
    },
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

impl AppState {
    pub fn get_noita(&mut self) -> Result<&mut Noita, ToolError> {
        match self.noita.as_mut() {
            Some(noita) => Ok(noita),
            None => ToolError::retry("Not connected to Noita"),
        }
    }
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

    #[serde(skip)]
    error: Option<ToolError>,
}

impl Pane {
    fn new(tool_info: &ToolInfo) -> Self {
        Self {
            title: tool_info.title.into(),
            tool: (tool_info.default_constructor)(),
            error: None,
        }
    }
}

fn default_tree() -> egui_tiles::Tree<Pane> {
    let mut tiles = egui_tiles::Tiles::default();

    // first tool is the process panel
    let (first, rest) = TOOLS.split_first().expect("No tools defined");

    let split_tab = vec![tiles.insert_pane(Pane::new(first))];

    let tabs = rest
        .iter()
        .map(|tool| tiles.insert_pane(Pane::new(tool)))
        .collect();

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
        if let Some(tile) = tiles.remove(tile_id) {
            match tile {
                Tile::Pane(pane) => {
                    self.hidden_tools.push(pane);
                }
                Tile::Container(container) => {
                    for tile_id in container.children() {
                        self.on_tab_close(tiles, *tile_id);
                    }
                }
            }
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
            loop {
                if let Some(e) = pane.error.as_ref() {
                    // bad state is informative, don't scream with red
                    let color = if matches!(e, ToolError::BadState(_)) {
                        ui.visuals().warn_fg_color
                    } else {
                        ui.visuals().error_fg_color
                    };

                    ui.label(RichText::new(e.to_string()).color(color));

                    if ui.button("Retry").clicked() {
                        pane.error = None;
                    }
                    break;
                }
                match pane.tool.ui(ui, self) {
                    Ok(()) => {}
                    Err(ToolError::ImmediateRetry(e)) => {
                        ui.label(format!("{e}"));
                    }
                    Err(e) => {
                        pane.error = Some(e);
                        continue; // goto drawing the error lol
                    }
                }
                break;
            }

            #[cfg(debug_assertions)]
            {
                use eframe::egui::{Align, Layout, RichText};

                ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
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

        #[cfg(debug_assertions)]
        {
            self.state.repaints += 1;
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        set_value(storage, eframe::APP_KEY, &self);
    }
}

impl NoitaUtilityBox {
    // in case of bugs or whatever that would cause tools to be lost from storage
    // or, more likely, new tools being added in new versions
    fn ensure_all_tools_present(&mut self) {
        let mut tools = TOOLS.iter().collect::<Vec<_>>();

        for tile in self.tree.tiles.tiles() {
            let Tile::Pane(pane) = tile else {
                continue;
            };
            tools.retain(|info| !info.is_it(&*pane.tool));
        }

        // also ensure there's no duplicates in hidden tools lol
        let mut unique_tools = HashSet::new();
        let prev = self.state.hidden_tools.len();
        self.state
            .hidden_tools
            .retain(|pane| unique_tools.insert(pane.tool.type_id()));
        let diff = prev - self.state.hidden_tools.len();
        if diff != 0 {
            tracing::info!("Removed {diff} duplicate hidden tools");
        }

        for tool in &self.state.hidden_tools {
            tools.retain(|info| !info.is_it(&*tool.tool));
        }
        if tools.is_empty() {
            return;
        }

        tracing::info!(
            "Restoring tools {:?} as hidden",
            tools.iter().map(|t| t.title).collect::<Vec<_>>()
        );
        self.state
            .hidden_tools
            .extend(tools.iter().map(|info| Pane::new(info)));
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
                egui_extras::install_image_loaders(&cc.egui_ctx);

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
