use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Context;
use eframe::egui::{
    self, CollapsingHeader, FontId, Grid, RichText, ScrollArea, TextStyle, Ui, Widget,
};
use noita_engine_reader::{
    CachedTranslations, PlayerState,
    memory::MemoryStorage,
    types::components::{ItemComponent, MaterialInventoryComponent},
};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

use super::{Result, Tool, ToolError};
use crate::app::AppState;
use crate::entities::wand::Wand;

#[derive(Debug, SmartDefault, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerInfo {
    realtime: bool,
    checked: HashSet<String>,
    auto_check: bool,
    #[serde(skip)]
    cached_translations: Arc<CachedTranslations>,
}

#[typetag::serde]
impl Tool for PlayerInfo {
    fn ui(&mut self, ui: &mut Ui, state: &mut AppState) -> Result {
        let noita = state.get_noita()?;

        ui.checkbox(&mut self.realtime, "Realtime");
        let refresh = ui.button("Refresh");

        if self.realtime
            || refresh.clicked()
            // The translation does not yet work, refresh it
            || self.cached_translations.translate("item_wand", false) == "item_wand"
        {
            ui.ctx().request_repaint();

            self.cached_translations = Arc::new(
                noita
                    .translations()
                    .context("Failed to read Noita translations")?,
            );
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

        let wand = noita.get_entity_tag_index("wand")?;
        let potion = noita.get_entity_tag_index("potion")?;
        let powder_stash = noita.get_entity_tag_index("powder_stash")?;

        let mut containers = Vec::new();
        let mut wands = Vec::new();

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
            } else if child.tags[wand] {
                let Some(item_comp) = store.get(&child)? else {
                    tracing::warn!(entity = child.id, "Wand has no ItemComponent?");
                    continue;
                };

                wands.push(("Wand", item_comp.inventory_slot, child));
            }
        }

        let store = noita.component_store::<MaterialInventoryComponent>()?;

        ScrollArea::both()
            .show(ui, |ui| {
                // Wand listing
                for (_, slot, wand_entity) in wands {
                    let wand = Wand::read_entity(&wand_entity, noita)
                        .context(format!("Reading wand in slot {slot:?}: {wand_entity:?}"))?;
                    let wand_name = wand.translated_name(&self.cached_translations);

                    let title = match slot.y {
                        0 => format!("{} (slot {})", wand_name, slot.x + 1),
                        y => format!("{} (slot x:{} y:{})", wand_name, slot.x + 1, y + 1),
                    };

                    CollapsingHeader::new(title)
                        .id_salt(format!("wands.{}", wand_entity.id))
                        .default_open(false)
                        .show(ui, |ui| {
                            WandWidget::new(wand_entity.id, wand).ui(ui);
                        });
                }

                // Potions and pouches
                for (name, slot, container) in containers {
                    let Ok(Some(mat_inv)) = store.get(&container) else {
                        continue;
                    };

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
                                        ui.label(format!("{amount:.2}"));
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

struct WandWidget {
    id: u32,
    wand: Wand,
}

impl WandWidget {
    fn new(id: u32, wand: Wand) -> WandWidget {
        WandWidget { id, wand }
    }
}

impl Widget for &WandWidget {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        Grid::new(format!("wands.{}.stats", self.id))
            .striped(true)
            .num_columns(2)
            .show(ui, |ui| {
                [
                    ("Sprite", self.wand.sprite.clone()),
                    (
                        "Shuffle",
                        if self.wand.shuffle_deck_when_empty {
                            "Yes"
                        } else {
                            "No"
                        }
                        .to_string(),
                    ),
                    ("Spells/Cast", self.wand.action_per_round.to_string()),
                    (
                        "Cast delay",
                        format!(
                            "{:0.2} s ({:0} f)",
                            self.wand.cast_delay as f32 / 60.0,
                            self.wand.cast_delay
                        ),
                    ),
                    (
                        "Rechrg. Time",
                        format!(
                            "{:0.2} s ({:0} f)",
                            self.wand.reload_time as f32 / 60.0,
                            self.wand.reload_time
                        ),
                    ),
                    (
                        "Mana",
                        format!("{:0.0}/{:0.0}", self.wand.mana, self.wand.mana_max),
                    ),
                    (
                        "Mana chg. Spd",
                        format!("{:0.0}", self.wand.mana_charge_speed),
                    ),
                    ("Capacity", self.wand.deck_capacity.to_string()),
                    ("Spread", format!("{:0.2} DEG", self.wand.spread)),
                ]
                .iter()
                .for_each(|(k, v)| {
                    ui.label(k.to_string());
                    ui.label(v);
                    ui.end_row();
                });
            });
        CollapsingHeader::new("Hidden Stats")
            .id_salt(format!("wands.{}.hidden_stats", self.id))
            .default_open(false)
            .show(ui, |ui| {
                Grid::new(format!("wands.{}.hidden_stats.table", self.id))
                    .striped(true)
                    .num_columns(2)
                    .show(ui, |ui| {
                        [("Speed Mult.", format!("{:0.1}", self.wand.speed_multipler))]
                            .iter()
                            .for_each(|(k, v)| {
                                ui.label(k.to_string());
                                ui.label(v);
                                ui.end_row();
                            });
                    });
            });

        let font = FontId::proportional(TextStyle::Body.resolve(ui.style()).size);
        let info = RichText::new("Wand Simulator").font(font);
        let url = self.wand.simulator_url();

        ui.hyperlink_to(info, url.clone()).on_hover_text(url);
        ui.end_row();

        ui.response()
    }
}
