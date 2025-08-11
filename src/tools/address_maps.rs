use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use derive_more::Debug;
use eframe::egui::{
    Align, Button, CollapsingHeader, Id, TextEdit, Ui, Vec2, Widget,
    collapsing_header::CollapsingState,
};
use egui_extras::{Column, TableBuilder};
use noita_engine_reader::{
    memory::{ProcessRef, Ptr, exe_image::ExeImage},
    noita::{NoitaGlobals, discovery},
};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

use crate::app::AppState;

use super::{Result, Tool};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AddressMapsData {
    maps: Vec<AddressMap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressEntry {
    name: String,
    address: u32,
    comment: String,
}

#[derive(SmartDefault, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AddressMapInner {
    name: String,
    noita_ts: u32,
    entries: Vec<AddressEntry>,
    #[default(Id::new(fastrand::u64(..)))]
    ui_id: Id,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct AddressMap(Arc<Mutex<AddressMapInner>>);

impl AddressMap {
    pub fn new(name: String, noita_ts: u32, entries: Vec<AddressEntry>) -> Self {
        Self(Arc::new(Mutex::new(AddressMapInner {
            name,
            noita_ts,
            entries,
            ui_id: Id::new(fastrand::u64(..)),
        })))
    }
}

impl AddressMap {
    fn get<T>(&self, name: &str) -> Option<Ptr<T>> {
        self.0
            .lock()
            .unwrap()
            .entries
            .iter()
            .find(|e| e.name == name)
            .map(|e| Ptr::of(e.address))
    }

    pub fn as_noita_globals(&self) -> NoitaGlobals {
        NoitaGlobals {
            world_seed: self.get("seed"),
            ng_count: self.get("ng-plus-count"),
            global_stats: self.get("global-stats"),
            game_global: self.get("game-global"),
            entity_manager: self.get("entity-manager"),
            entity_tag_manager: self.get("entity-tag-manager"),
            component_type_manager: self.get("component-type-manager"),
            translation_manager: self.get("translation-manager"),
            platform: self.get("platform"),
        }
    }
}

fn hex_input(value: &mut u32) -> impl Widget + '_ {
    move |ui: &mut Ui| {
        let mut ts = format!("0x{value:x}");
        let response = ui.add(
            TextEdit::singleline(&mut ts)
                .horizontal_align(Align::Center)
                .desired_width(75.0),
        );
        // allow text input to be empty
        if ts.is_empty() {
            *value = 0;
        } else if let Ok(ts) = ts.parse() {
            *value = ts;
        } else if let Some(ts) = ts.strip_prefix("0x").and_then(|ts| {
            // allow typing in 0x
            if ts.is_empty() {
                Some(0)
            } else {
                u32::from_str_radix(ts, 16).ok()
            }
        }) {
            *value = ts;
        }
        response
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AddressMaps;

#[typetag::serde]
impl Tool for AddressMaps {
    fn ui(&mut self, ui: &mut Ui, state: &mut AppState) -> Result {
        let mut removed = None;

        let s = &mut state.address_maps;

        for (i, map) in s.maps.iter_mut().enumerate() {
            let mut map = map.0.lock().unwrap();
            CollapsingHeader::new(format!("(0x{:x}) {}", map.noita_ts, map.name))
                .id_salt(map.ui_id)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Executable timestamp: ");
                        ui.add(hex_input(&mut map.noita_ts));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Comment: ");
                        ui.text_edit_singleline(&mut map.name);
                    });

                    // oof
                    let header_id = ui
                        .stack()
                        .parent
                        .as_ref()
                        .unwrap()
                        .parent
                        .as_ref()
                        .unwrap()
                        .id
                        .with(map.ui_id);

                    let confirm_id = ui.make_persistent_id("confirm");

                    let confirm = ui.data(|d| d.get_temp(confirm_id).unwrap_or_default());

                    if confirm {
                        ui.horizontal(|ui| {
                            ui.label("Are you sure?");
                            if ui.button("Yes").clicked() {
                                ui.data_mut(|d| d.remove::<bool>(confirm_id));
                                removed = Some((i, header_id));
                            }
                            if ui.button("No").clicked() {
                                ui.data_mut(|d| d.remove::<bool>(confirm_id));
                            }
                        });
                    } else if ui.button("Delete").clicked() {
                        ui.data_mut(|d| d.insert_temp(confirm_id, true));
                    }

                    ui.separator();

                    ui.vertical(|ui| {
                        TableBuilder::new(ui)
                            .striped(true)
                            .column(Column::auto())
                            .column(Column::auto().resizable(true))
                            .column(Column::auto())
                            .column(Column::remainder().clip(true))
                            .header(20.0, |mut header| {
                                header.col(|_| {});
                                header.col(|ui| {
                                    ui.label("Name");
                                });
                                header.col(|ui| {
                                    ui.label("Address");
                                });
                                header.col(|ui| {
                                    ui.label("Comment");
                                });
                            })
                            .body(|mut body| {
                                let mut removed = None;
                                for (i, entry) in map.entries.iter_mut().enumerate() {
                                    let AddressEntry {
                                        name,
                                        address,
                                        comment,
                                    } = entry;

                                    body.row(20.0, |mut row| {
                                        row.col(|ui| {
                                            if ui
                                                .add(Button::new(" -").min_size(Vec2::splat(18.0)))
                                                .clicked()
                                            {
                                                removed = Some(i);
                                            }
                                        });
                                        row.col(|ui| {
                                            ui.add_space(0.5);
                                            ui.add(TextEdit::singleline(name));
                                            ui.add_space(0.5);
                                        });
                                        row.col(|ui| {
                                            ui.add_space(0.5);
                                            ui.add(hex_input(address));
                                            ui.add_space(0.5);
                                        });
                                        row.col(|ui| {
                                            ui.add_space(0.5);
                                            ui.add(TextEdit::singleline(comment));
                                            ui.add_space(0.5);
                                        });
                                    });
                                }
                                if let Some(i) = removed {
                                    map.entries.remove(i);
                                }
                                body.row(20.0, |mut row| {
                                    row.col(|ui| {
                                        if ui
                                            .add(Button::new(" +").min_size(Vec2::splat(18.0)))
                                            .clicked()
                                        {
                                            map.entries.push(AddressEntry {
                                                name: "new".to_owned(),
                                                address: 0,
                                                comment: String::new(),
                                            });
                                        }
                                    });
                                })
                            })
                    });
                });
        }

        if let Some((i, header_id)) = removed {
            s.maps.remove(i);

            // cleanup the collapsing state at this id
            CollapsingState::load_with_default_open(ui.ctx(), header_id, false).remove(ui.ctx());
            ui.ctx().animate_bool_with_time(header_id, false, 0.0);
        }

        if ui.button("Add").clicked() {
            s.maps.push(AddressMap::default());
        }

        Ok(())
    }
}

impl AddressMapsData {
    pub fn get(&self, noita_ts: u32) -> Option<AddressMap> {
        self.maps
            .iter()
            .find(|m| m.0.lock().unwrap().noita_ts == noita_ts)
            .cloned()
    }

    pub fn discover(&mut self, proc: &ProcessRef) -> anyhow::Result<()> {
        fn add_entry<T>(
            entries: &mut Vec<AddressEntry>,
            name: &str,
            ptr: Option<Ptr<T>>,
            comment: &str,
        ) {
            if let Some(ptr) = ptr {
                entries.push(AddressEntry {
                    name: name.to_owned(),
                    address: ptr.addr(),
                    comment: comment.to_owned(),
                });
            } else {
                tracing::warn!("{name} pointer not found");
            }
        }

        let image = ExeImage::read(proc)
            .context("Reading the entire EXE image of the game for discovery")?;

        let NoitaGlobals {
            world_seed,
            ng_count,
            global_stats,
            game_global,
            entity_manager,
            entity_tag_manager,
            component_type_manager,
            translation_manager,
            platform,
        } = discovery::run(&image);

        let mut entries = Vec::new();
        add_entry(&mut entries, "seed", world_seed, "Current world seed");
        add_entry(
            &mut entries,
            "ng-plus-count",
            ng_count,
            "New Game Plus counter",
        );
        add_entry(
            &mut entries,
            "global-stats",
            global_stats,
            "Used to get all the stats",
        );
        add_entry(
            &mut entries,
            "game-global",
            game_global,
            "Stores global game state, like the list of materials",
        );
        add_entry(
            &mut entries,
            "entity-manager",
            entity_manager,
            "Entity manager, used to find the player or whatever it got polymorphed into",
        );
        add_entry(
            &mut entries,
            "entity-tag-manager",
            entity_tag_manager,
            "Entity tag manager, also used to find the player",
        );
        add_entry(
            &mut entries,
            "component-type-manager",
            component_type_manager,
            "Component type manager, used to get entity components",
        );
        add_entry(
            &mut entries,
            "translation-manager",
            translation_manager,
            "Allows us to get localized strings from the game, such as the material names",
        );
        add_entry(
            &mut entries,
            "platform",
            platform,
            "Platform-specific stuff, only used to get the game install directory",
        );

        if !entries.is_empty() {
            let name = match discovery::find_noita_build(&image) {
                Some(noita) => format!("Autodiscovered - {noita}"),
                None => "Autodiscovered (no noita build string found!)".into(),
            };

            self.maps
                .push(AddressMap::new(name, proc.header().timestamp(), entries));
        }

        Ok(())
    }
}
