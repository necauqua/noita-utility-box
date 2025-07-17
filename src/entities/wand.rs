use std::sync::Arc;

use anyhow::{Context, Result};
use noita_engine_reader::{
    CachedTranslations, Noita,
    memory::MemoryStorage,
    types::{
        Entity,
        components::{AbilityComponent, ItemActionComponent, ItemComponent},
    },
};

pub struct Wand {
    pub name: String,
    pub sprite: String,
    pub action_per_round: i32,
    pub deck_capacity: i32,
    pub mana: f32,
    pub mana_max: f32,
    pub mana_charge_speed: f32,
    pub cast_delay: i32,
    pub spread: f32,
    pub reload_time: i32,
    pub shuffle_deck_when_empty: bool,
    pub spells: Vec<String>,
    pub speed_multipler: f32,
}

impl Wand {
    pub fn read_entity(entity: &Entity, noita: &mut Noita) -> Result<Wand> {
        let p = noita.proc().clone();

        let item = noita
            .component_store::<ItemComponent>()
            .context("Failed to acquire ItemComponent store")?
            .get(entity)
            .context("Failed to read ItemComponent of wand")?
            .unwrap();

        let item_name = item
            .item_name
            .read(&p)
            .context("Failed to read localized wand name")?
            .trim_start_matches("$")
            .to_string();

        // NOTE: Daily practice does not initialize wands with the name "item_wand"
        let name = if !item_name.is_empty() {
            item_name
        } else {
            "item_wand".to_string()
        };

        let ability = noita
            .component_store::<AbilityComponent>()
            .context("Failed to acquire AbilityComponent store")?
            .get(entity)
            .context("Failed to read AbilityComponent of wand")?
            .unwrap();

        let sprite = ability
            .sprite_file
            .read(&p)
            .context("Failed to read wand sprite filename")?;

        // Reading the spells on the wand
        let spell_entities = entity
            .children
            .read(&p)
            .context(format!("Failed to read spells of wand: {entity:?}"))?
            .read_storage(&p)
            .context(format!("Failed to read spells of wand: {entity:?}"))?;

        let spells_store = noita
            .component_store::<ItemActionComponent>()
            .context("Failed to acquire ItemActionComponent store")?;

        let mut spells = Vec::new();
        for spell_entity in spell_entities {
            let Some(spell_entity) = spells_store.get(&spell_entity).context(format!(
                "Failed to read ActionComponent of wand: {spell_entity:?}"
            ))?
            else {
                continue;
            };

            let spell_name = spell_entity
                .action_id
                .read(&p)
                .context(format!("Failed to read name of spell: {spell_entity:?}"))?;

            spells.push(spell_name);
        }
        Ok(Wand {
            name,
            sprite,
            action_per_round: ability.gun_config.actions_per_round,
            deck_capacity: ability.gun_config.deck_capacity,
            mana: ability.mana,
            mana_max: ability.mana_max,
            mana_charge_speed: ability.mana_charge_speed,
            cast_delay: ability.gunaction_config.fire_rate_wait,
            reload_time: ability.gun_config.reload_time,
            shuffle_deck_when_empty: ability.gun_config.shuffle_deck_when_empty.as_bool(),
            spread: ability.gunaction_config.spread_degrees,
            spells,
            speed_multipler: ability.gunaction_config.speed_multiplier,
        })
    }

    pub fn translated_name(&self, translations: &Arc<CachedTranslations>) -> String {
        let translated_name = translations.translate(&self.name, true).to_string();

        // Non-translated are just "Wand" (Or with wand with empty name)
        if self.name == translated_name || self.name.is_empty() {
            translations.translate("item_wand", true).to_string()
        } else {
            translated_name
        }
    }

    pub fn simulator_url(&self) -> String {
        format!(
            "https://noita-wand-simulator.salinecitrine.com/?{}",
            [
                ("action_per_round", self.action_per_round.to_string()),
                ("deck_capacity", self.deck_capacity.to_string()),
                ("cast_delay", self.cast_delay.to_string()),
                ("reload_time", self.reload_time.to_string()),
                (
                    "shuffle_deck_when_empty",
                    self.shuffle_deck_when_empty.to_string(),
                ),
                ("mana_max", self.mana_max.to_string()),
                ("mana_charge_speed", self.mana_charge_speed.to_string()),
                ("spread", format!("{:0.0}", self.spread)),
                ("spells", self.spells.join(",")),
            ]
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<String>>()
            .join("&")
        )
    }
}
