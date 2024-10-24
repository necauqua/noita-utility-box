use std::collections::HashSet;

use anyhow::Context;
use eframe::egui::{CollapsingHeader, Grid, ScrollArea, Ui};
use noita_utility_box::{
    memory::MemoryStorage,
    noita::types::components::{ItemComponent, MaterialInventoryComponent},
};
use serde::{Deserialize, Serialize};

use crate::app::AppState;

use super::{Result, Tool};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MaterialPipette {
    realtime: bool,
    checked: HashSet<String>,
    auto_check: bool,
}

#[typetag::serde]
impl Tool for MaterialPipette {
    fn ui(&mut self, ui: &mut Ui, state: &mut AppState) -> Result {
        self.ui(ui, state);
        Ok(())
    }
}

impl MaterialPipette {
    pub fn ui(&mut self, ui: &mut Ui, state: &mut AppState) {
        ui.checkbox(&mut self.realtime, "Realtime");
        if self.realtime {
            ui.ctx().request_repaint();
        }

        ui.separator();

        let Some(noita) = state.noita.as_mut() else {
            ui.label("Noita not connected");
            return;
        };

        // just do it on every redraw, whatever (todo add at least a timer here lol)
        let res = (|| {
            let player = match noita.get_player()? {
                Some((player, false)) => player,
                Some((_, true)) => {
                    ui.label("Polymorphed LOL");
                    return anyhow::Ok(());
                }
                None => {
                    ui.label("Player not found");
                    return Ok(());
                }
            };

            let p = noita.proc().clone();

            let mut inv_quick = None;
            for child in player.children.read(&p)?.read(&p)? {
                let child = child.read(&p)?;
                if child.name.read(&p)? == "inventory_quick" {
                    inv_quick = Some(child);
                    break;
                }
            }
            let inv_quick = inv_quick.context("Player has no inventory?")?;

            let potion = noita.get_entity_tag_index("potion")?;
            let powder_stash = noita.get_entity_tag_index("powder_stash")?;

            let mut containers = Vec::new();

            let store = noita.component_store::<ItemComponent>()?;

            for child in inv_quick.children.read(&p)?.read(&p)? {
                let child = child.read(&p)?;

                if child.tags[potion] {
                    let Some(item_comp) = store.get(&child)? else {
                        tracing::warn!(entity = child.id, "Potion has no ItemComponent?");
                        continue;
                    };

                    containers.push(("Flask", item_comp.inventory_slot, child));
                } else if child.tags[powder_stash] {
                    let Some(item_comp) = store.get(&child)? else {
                        tracing::warn!(entity = child.id, "Flask has no ItemComponent?");
                        continue;
                    };

                    containers.push(("Pouch", item_comp.inventory_slot, child));
                }
            }

            let store = noita.component_store::<MaterialInventoryComponent>()?;

            ScrollArea::both()
                .show(ui, |ui| {
                    for (name, slot, container) in containers {
                        let mat_inv = store
                            .get(&container)?
                            .context("Container has no MaterialInventoryComponent?")?;

                        let mats = mat_inv
                            .count_per_material_type
                            .read(&p)?
                            .into_iter()
                            .enumerate()
                            .filter_map(|(i, f)| (f > 0.0).then_some((i as u32, f)))
                            .collect::<Vec<_>>();

                        let title = match slot.y {
                            0 => format!("{name} (slot {})", slot.x + 1),
                            y => format!("{name} (slot x:{} y:{})", slot.x + 1, y + 1),
                        };

                        CollapsingHeader::new(title)
                            .id_salt(container.id) // whatever lul
                            .default_open(true)
                            .show(ui, |ui| {
                                Grid::new(container.id)
                                    .num_columns(2)
                                    .show(ui, |ui| {
                                        if mats.is_empty() {
                                            ui.label("<Empty>");
                                            ui.end_row();
                                            return anyhow::Ok(());
                                        }
                                        for (idx, amount) in mats {
                                            let name =
                                                noita.get_material_name(idx)?.unwrap_or_else(
                                                    || format!("unknown material (index {idx})"),
                                                );
                                            ui.label(format!("{name:?}"));
                                            ui.label(format!("{:.2}", amount));
                                            ui.end_row();

                                            if self.auto_check {
                                                self.checked.insert(name);
                                            }
                                        }
                                        Ok(())
                                    })
                                    .inner
                            })
                            .body_returned
                            .transpose()?;
                    }

                    ui.separator();

                    CollapsingHeader::new("Material checklist")
                        .show(ui, |ui| {
                            ui.checkbox(&mut self.auto_check, "Automatically check held materials");
                            if ui.button("Reset").clicked() {
                                self.checked.clear();
                            }
                            ui.add_space(0.5);
                            Grid::new("all_materials")
                                .num_columns(4)
                                .striped(true)
                                .show(ui, |ui| {
                                    for idx in 0..noita.materials()?.len() as u32 {
                                        let name = noita.get_material_name(idx)?.unwrap();

                                        ui.label(idx.to_string());

                                        let mut checked = self.checked.contains(&name);
                                        ui.checkbox(&mut checked, "");

                                        ui.label(format!("{name:?}"));

                                        if checked {
                                            self.checked.insert(name);
                                        } else {
                                            self.checked.remove(&name);
                                        }

                                        if let Some(ui_name) = noita.get_material_ui_name(idx)? {
                                            ui.label(ui_name);
                                        }

                                        ui.end_row();
                                    }
                                    Ok(())
                                })
                                .inner
                        })
                        .body_returned
                        .transpose()
                        .map(|_| ())
                })
                .inner
        })();

        if let Err(e) = res {
            ui.label(format!("Error: {e}"));
        }
    }
}
