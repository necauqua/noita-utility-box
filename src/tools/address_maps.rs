use std::{
    io,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use derive_more::Debug;
use eframe::egui::{
    collapsing_header::CollapsingState, Align, Button, CollapsingHeader, Id, TextEdit, Ui, Vec2,
    Widget,
};
use egui_extras::{Column, TableBuilder};
use noita_utility_box::{
    memory::{exe_image::PeHeader, ProcessRef, Ptr},
    noita::{discovery, NoitaGlobals},
};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AddressMaps {
    maps: Vec<AddressMap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AddressEntry {
    name: String,
    address: u32,
    comment: String,
}

#[derive(SmartDefault, Debug, Serialize, Deserialize)]
pub struct AddressMapData {
    name: String,
    noita_ts: u32,
    entries: Vec<AddressEntry>,
    #[default(Id::new(fastrand::u64(..)))]
    ui_id: Id,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct AddressMap {
    data: Arc<RwLock<AddressMapData>>,
}

impl AddressMap {
    pub fn data(&self) -> RwLockReadGuard<AddressMapData> {
        self.data.read().unwrap()
    }

    pub fn data_mut(&self) -> RwLockWriteGuard<AddressMapData> {
        self.data.write().unwrap()
    }

    fn get<T>(&self, name: &str) -> Option<Ptr<T>> {
        self.data()
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
        }
    }
}

fn hex_input(value: &mut u32) -> impl Widget + '_ {
    move |ui: &mut Ui| {
        let mut ts = format!("0x{:x}", value);
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

impl AddressMaps {
    pub fn get(&self, noita_ts: u32) -> Option<AddressMap> {
        self.maps
            .iter()
            .find(|m| m.data.read().unwrap().noita_ts == noita_ts)
            .cloned()
    }

    pub fn discover(&mut self, proc: &ProcessRef, header: &PeHeader) -> io::Result<()> {
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

        let image = header.clone().read_image(proc)?;
        let discovered = discovery::run(&image);

        let mut entries = Vec::new();
        add_entry(
            &mut entries,
            "seed",
            discovered.world_seed,
            "Current world seed",
        );
        add_entry(
            &mut entries,
            "ng-plus-count",
            discovered.ng_count,
            "New Game Plus counter",
        );
        add_entry(
            &mut entries,
            "global-stats",
            discovered.global_stats,
            "Used to get all the stats",
        );
        add_entry(
            &mut entries,
            "game-global",
            discovered.game_global,
            "Stores global game state, like the list of materials",
        );
        add_entry(
            &mut entries,
            "entity-manager",
            discovered.entity_manager,
            "Entity manager, used to find the player or whatever it got polymorphed into",
        );
        add_entry(
            &mut entries,
            "entity-tag-manager",
            discovered.entity_tag_manager,
            "Entity tag manager, also used to find the player",
        );
        add_entry(
            &mut entries,
            "component-type-manager",
            discovered.component_type_manager,
            "Component type manager, used to get entity components",
        );

        if !entries.is_empty() {
            let name = match discovery::find_noita_build(&image) {
                Some(noita) => format!("Autodiscovered - {noita}"),
                None => "Autodiscovered (no noita build string found!)".into(),
            };

            self.maps.push(AddressMap {
                data: Arc::new(RwLock::new(AddressMapData {
                    name,
                    noita_ts: header.timestamp(),
                    entries,
                    ui_id: Id::new(fastrand::u64(..)),
                })),
            });
        }

        Ok(())
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("Address Maps");
        ui.separator();

        let mut removed = None;

        for (i, map) in self.maps.iter_mut().enumerate() {
            let mut map = map.data_mut();
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
            self.maps.remove(i);

            // cleanup the collapsing state at this id
            CollapsingState::load_with_default_open(ui.ctx(), header_id, false).remove(ui.ctx());
            ui.ctx().animate_bool_with_time(header_id, false, 0.0);
        }

        if ui.button("Add").clicked() {
            self.maps.push(AddressMap::default());
        }
    }
}
