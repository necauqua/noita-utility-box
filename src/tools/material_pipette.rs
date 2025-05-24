use std::collections::HashSet;

use anyhow::Context;
use eframe::egui::{CollapsingHeader, Grid, ScrollArea, Ui};
use noita_engine_reader::{
    PlayerState,
    memory::MemoryStorage,
    types::components::{ItemComponent, MaterialInventoryComponent},
};
use serde::{Deserialize, Serialize};

use crate::app::AppState;

use super::{Result, Tool, ToolError};

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
        let noita = state.get_noita()?;

        ui.checkbox(&mut self.realtime, "Realtime");
        if self.realtime {
            ui.ctx().request_repaint();
        }

        ui.separator();

        // just do it all on every redraw, whatever (todo add at least a timer here lol)
        let player = match noita.get_player()? {
            Some((_, PlayerState::Polymorphed)) => {
                ui.label("Polymorphed LOL");
                return Ok(());
            }
            Some((player, _)) => player,
            None => return ToolError::retry("Player entity not found"),
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
                                            noita.get_material_name(idx)?.unwrap_or_else(|| {
                                                format!("unknown material (index {idx})")
                                            });
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
                Ok(())
            })
            .inner
    }
}
