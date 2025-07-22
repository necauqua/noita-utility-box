use std::{f32::consts::TAU, sync::Arc};

use anyhow::Context;
use eframe::egui::{
    CollapsingHeader, Frame, Grid, Image, OpenUrl, ScrollArea, TextureOptions, Ui, Vec2, Widget,
};
use noita_engine_reader::{
    CachedTranslations, ComponentStore, Noita, PlayerState,
    memory::MemoryStorage,
    types::{
        Entity, Vec2i,
        components::{
            AbilityComponent, DamageModelComponent, ItemActionComponent, ItemComponent,
            MaterialInventoryComponent,
        },
    },
};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

use super::{Result, Tool, ToolError};
use crate::{app::AppState, tools::ComponentStoreExt};

#[derive(Debug, SmartDefault, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerInfo {
    realtime: bool,
    #[default(true)]
    multiply_hp: bool,
    #[serde(skip)]
    cached_translations: Arc<CachedTranslations>,
}

impl PlayerInfo {
    fn read_item_name(
        &mut self,
        noita: &mut Noita,
        store: &ComponentStore<ItemComponent>,
        entity: &Entity,
        default_name: &str,
    ) -> Result<String> {
        let item_name = store.get_checked(entity)?.item_name.read(noita.proc())?;

        // NOTE: Daily practice does not initialize wands with the name "item_wand"
        let key = match &*item_name {
            "" => default_name,
            n => n.trim_start_matches('$'),
        };
        let translated = self.cached_translations.translate(key, true);
        if translated != key {
            return Ok(translated.into_owned());
        }
        Ok(self
            .cached_translations
            .translate(default_name, true)
            .into_owned())
    }
}

fn section(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui) -> Result) -> Result {
    CollapsingHeader::new(title)
        .show(ui, |ui| {
            ScrollArea::both()
                .auto_shrink([false, true])
                .show(ui, add_contents)
                .inner
        })
        .body_returned
        .transpose()
        .unwrap_or_default();
    Ok(())
}

#[typetag::serde]
impl Tool for PlayerInfo {
    fn ui(&mut self, ui: &mut Ui, state: &mut AppState) -> Result {
        let noita = state.get_noita()?;

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.realtime, "Realtime");
            if self.realtime {
                ui.ctx().request_repaint();
            }

            if ui.button("Refresh").clicked() || self.cached_translations.is_empty() {
                self.cached_translations = Arc::new(
                    noita
                        .translations()
                        .context("Failed to read language data")?,
                );
            }
            Result::Ok(())
        })
        .inner?;

        ui.separator();

        let player = match noita.get_player()? {
            Some((_, PlayerState::Polymorphed)) => {
                ui.label("Polymorphed LOL");
                return Ok(());
            }
            Some((player, PlayerState::Normal)) => player,
            _ => return ToolError::retry("Player entity not found"),
            // ^ cessated entity is empty so it wont have inventory_quick etc, pretend it doesn't exist
        };

        let p = noita.proc().clone();
        let p = &p;

        let inv_quick = player
            .first_child_by_name("inventory_quick", p)
            .context("Reading inventory_quick child entity")?
            .context("Player had no inventory_quick")?;

        let wand = noita.get_entity_tag_index("wand")?;
        let potion = noita.get_entity_tag_index("potion")?;
        let powder_stash = noita.get_entity_tag_index("powder_stash")?;

        let mut containers = Vec::new();
        let mut wands = Vec::new();

        let item_store = noita.component_store::<ItemComponent>()?;

        for child in inv_quick.children.read(p)?.read_storage(p)? {
            if child.tags[potion] || child.tags[powder_stash] {
                containers.push((item_store.get_checked(&child)?.inventory_slot, child));
            } else if child.tags[wand] {
                wands.push(child);
            }
        }

        let dmc_store = noita.component_store::<DamageModelComponent>()?;
        let ability_store = noita.component_store::<AbilityComponent>()?;
        let item_store = noita.component_store::<ItemComponent>()?;
        let spell_store = noita.component_store::<ItemActionComponent>()?;
        let mat_store = noita.component_store::<MaterialInventoryComponent>()?;

        section(ui, "Wands", |ui| {
            ui.horizontal(|ui| {
                for entity in wands {
                    ui.vertical(|ui| {
                        ui.add(
                            &Wand::read(
                                noita,
                                &ability_store,
                                &item_store,
                                &spell_store,
                                &self.cached_translations,
                                &entity,
                            )
                            .context(format!("Reading wand {entity:?}"))?,
                        );
                        Result::Ok(())
                    })
                    .inner?;
                }
                Result::Ok(())
            })
            .inner
        })?;

        section(ui, "Inventory Materials", |ui| {
            for (slot, entity) in containers {
                let item = MaterialStorageItem::read(noita, &mat_store, &entity)?;

                let name = self
                    .read_item_name(noita, &item_store, &entity, "item_empty")
                    .context("Reading item name")?;

                let title = match slot.y {
                    0 => format!("{name} (slot {})", slot.x + 1),
                    y => format!("{name} (slot x:{} y:{})", slot.x + 1, y + 1),
                };

                CollapsingHeader::new(title)
                    .default_open(true)
                    .show(ui, |ui| ui.add(&item));
            }
            Result::Ok(())
        })?;

        section(ui, "Player Damage", |ui| {
            let dmc = dmc_store.get_checked(&player)?;
            ui.checkbox(
                &mut self.multiply_hp,
                "Multiply HP value by 25 (like Noita UI does)",
            );
            Grid::new(ui.id().with("grid"))
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Current HP");
                    ui.label(format!(
                        "{}",
                        if self.multiply_hp {
                            dmc.hp.get() * 25.0
                        } else {
                            dmc.hp.get() // dont even multiply by 1 just in case
                        }
                    ));
                    ui.end_row();

                    ui.label("Max HP");
                    ui.label(format!(
                        "{}",
                        if self.multiply_hp {
                            dmc.max_hp.get() * 25.0
                        } else {
                            dmc.max_hp.get()
                        }
                    ));
                    ui.end_row();

                    ui.label("Curse damage");
                    ui.label(format!(
                        "{}",
                        if self.multiply_hp {
                            dmc.hp.get() * 100.0 * 25.0
                        } else {
                            dmc.hp.get() * 100.0
                        }
                    ));
                    ui.end_row();
                });
            CollapsingHeader::new("Damage Multipliers").show(ui, |ui| {
                ui.small("Damage multipliers equal to 1.0 are omitted");
                Grid::new(ui.id().with("grid"))
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        serde_json::to_value(&dmc.damage_multipliers)
                            .unwrap() // never fails
                            .as_object()
                            .unwrap() // never fails
                            .iter()
                            .for_each(|(key, value)| {
                                if value == 1.0 {
                                    return;
                                }
                                ui.label(key);
                                ui.label(value.to_string());
                                ui.end_row();
                            });
                    });
            });
            Result::Ok(())
        })?;

        Ok(())
    }
}

#[derive(Debug)]
struct Wand {
    id: u32,
    name: String,
    slot: Vec2i,
    ability: AbilityComponent,
    sprite: Option<(String, Arc<[u8]>)>,
    spells: Vec<String>,
    always_cast_count: i32,
}

impl Wand {
    fn read(
        noita: &mut Noita,
        store: &ComponentStore<AbilityComponent>,
        item_store: &ComponentStore<ItemComponent>,
        spell_store: &ComponentStore<ItemActionComponent>,
        translations: &CachedTranslations,
        entity: &Entity,
    ) -> Result<Self> {
        let item_component = item_store.get_checked(entity)?;
        let item_name = item_component.item_name.read(noita.proc())?;

        // NOTE: Daily practice does not initialize wands with the name "item_wand"
        let key = match &*item_name {
            "" => "item_wand",
            n => n.trim_start_matches('$'),
        };
        let translated = translations.translate(key, true);
        let name = if translated != key {
            translated.into_owned()
        } else {
            translations.translate("item_wand", true).into_owned()
        };

        let ability = store.get_checked(entity)?;

        let sprite_file = ability.sprite_file.read(noita.proc())?;
        let sprite = match &*sprite_file {
            "" => None,
            s => {
                // just cheat, for wands this seems ok
                let s = s.replace(".xml", ".png");
                Some((format!("bytes://{s}"), noita.get_file(&s)?))
            }
        };

        let p = noita.proc();
        let mut spells = Vec::new();
        let mut always_cast_count = 0;

        for entity in entity.children.read(p)?.read_storage(p)? {
            let item = item_store.get_checked(&entity)?;
            let spell = spell_store.get_checked(&entity)?;

            spells.push(spell.action_id.read(p)?);
            always_cast_count += item.permanently_attached.as_bool() as i32;
        }

        Ok(Self {
            id: entity.id,
            name,
            slot: item_component.inventory_slot,
            ability,
            sprite,
            spells,
            always_cast_count,
        })
    }

    fn simulator_url(&self) -> String {
        format!(
            concat!(
                "https://noita-wand-simulator.salinecitrine.com/?",
                "actions_per_round={}&",
                "deck_capacity={}&",
                "cast_delay={}&",
                "reload_time={}&",
                "shuffle_deck_when_empty={}&",
                "mana_max={}&",
                "mana_charge_speed={}&",
                "spread={:0.0}&",
                "spells={}",
            ),
            self.ability.gun_config.actions_per_round,
            self.ability.gun_config.deck_capacity,
            self.ability.gunaction_config.fire_rate_wait,
            self.ability.gun_config.reload_time,
            self.ability.gun_config.shuffle_deck_when_empty,
            self.ability.mana_max,
            self.ability.mana_charge_speed,
            self.ability.gunaction_config.spread_degrees,
            self.spells.join(","),
        )
    }
}

impl Widget for &Wand {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        Frame::group(ui.style())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(&self.name);
                    ui.small(match self.slot.y {
                        0 => format!("(slot {})", self.slot.x + 1),
                        y => format!("(slot x:{} y:{})", self.slot.x + 1, y + 1),
                    });
                });

                let id = ui.id().with(self.id);

                ui.horizontal(|ui| {
                    Grid::new(id.with("stats"))
                        .striped(true)
                        .num_columns(2)
                        .show(ui, |ui| {
                            let shuffle = self.ability.gun_config.shuffle_deck_when_empty.as_bool();
                            ui.label("Shuffle");
                            ui.label(if shuffle { "Yes" } else { "No" });
                            ui.end_row();

                            ui.label("Spells/Cast");
                            ui.label(self.ability.gun_config.actions_per_round.to_string());
                            ui.end_row();

                            let delay = self.ability.gunaction_config.fire_rate_wait;
                            ui.label("Cast delay");
                            ui.label(format!("{:.2} s ({delay} f)", delay as f32 / 60.0));
                            ui.end_row();

                            let time = self.ability.gun_config.reload_time;
                            ui.label("Rechrg. Time");
                            ui.label(format!("{:.2} s ({time} f)", time as f32 / 60.0));
                            ui.end_row();

                            ui.label("Mana");
                            ui.label(if self.ability.mana == self.ability.mana_max {
                                format!("{:.0}", self.ability.mana_max)
                            } else {
                                format!("{:.0}/{:.0}", self.ability.mana, self.ability.mana_max)
                            });
                            ui.end_row();

                            ui.label("Mana chg. Spd");
                            ui.label(format!("{:.0}", self.ability.mana_charge_speed));
                            ui.end_row();

                            ui.label("Capacity");
                            ui.label(match self.always_cast_count {
                                0 => self.ability.gun_config.deck_capacity.to_string(),
                                1 => format!(
                                    "{} (+1 always cast)",
                                    self.ability.gun_config.deck_capacity - 1
                                ),
                                _ => format!(
                                    "{} (+{} always casts)",
                                    self.ability.gun_config.deck_capacity - self.always_cast_count,
                                    self.always_cast_count,
                                ),
                            });
                            ui.end_row();

                            ui.label("Spread");
                            ui.label(format!(
                                "{:.2} DEG",
                                self.ability.gunaction_config.spread_degrees
                            ));
                            ui.end_row();
                        });

                    if let Some(sprite) = &self.sprite {
                        ui.add(
                            Image::new(sprite.clone())
                                .rotate(-TAU / 4.0, Vec2::new(0.5, 0.5))
                                .fit_to_original_size(ui.pixels_per_point() * 1.2)
                                .texture_options(TextureOptions::NEAREST),
                        );
                    }
                });

                CollapsingHeader::new("Hidden Stats")
                    .id_salt(id.with("hidden"))
                    .default_open(false)
                    .show(ui, |ui| {
                        Grid::new(ui.id().with("grid"))
                            .striped(true)
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Speed Mult.");
                                ui.label(format!(
                                    "{:0.2}",
                                    self.ability.gunaction_config.speed_multiplier
                                ));
                                ui.end_row();
                                ui.label("Entity ID");
                                ui.label(format!("{}", self.id));
                                ui.end_row();
                            });
                    });

                let sim = ui
                    .button("Wand Simulator")
                    .on_hover_text("Includes the spells currently on the wand");

                if sim.clicked() {
                    ui.ctx().open_url(OpenUrl::new_tab(self.simulator_url()));
                }
            })
            .response
    }
}

#[derive(Debug)]
struct MaterialStorageItem {
    materials: Vec<(String, f64)>,
}

impl MaterialStorageItem {
    fn read(
        noita: &mut Noita,
        store: &ComponentStore<MaterialInventoryComponent>,
        entity: &Entity,
    ) -> Result<Self> {
        let comp = store.get_checked(entity)?;
        let p = noita.proc().clone();
        let materials = noita.materials()?;
        let materials = comp
            .count_per_material_type
            .read(&p)?
            .into_iter()
            .enumerate()
            .filter_map(|(i, f)| {
                if f == 0.0 {
                    return None;
                }
                materials.get(i).map(|m| (m.clone(), f))
            })
            .collect::<Vec<_>>();

        Ok(Self { materials })
    }
}

impl Widget for &MaterialStorageItem {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        Grid::new(ui.id().with("grid"))
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                if self.materials.is_empty() {
                    ui.label("<Empty>");
                    ui.end_row();
                }
                for (id, amount) in &self.materials {
                    ui.label(format!("{id:?}"));
                    ui.label(format!("{amount:.2}"));
                    ui.end_row();
                }
            })
            .response
    }
}
