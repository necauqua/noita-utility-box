#![recursion_limit = "256"]

use std::{collections::HashMap, time::Duration};

use anyhow::{Context, Result};
use noita_engine_reader::{
    memory::{MemoryStorage, Ptr, RawPtr, StdString, StdVec},
    rng::NoitaRng,
    types::{
        components::{
            AbilityComponent, DamageModelComponent, ItemComponent, LuaComponent, UIIconComponent,
        },
        platform::FileDevice,
    },
};
use rayon::iter::{IndexedParallelIterator as _, IntoParallelIterator, ParallelIterator};

mod common;

#[test]
#[ignore]
fn read_hp() -> Result<()> {
    let mut noita = common::setup()?;
    let store = noita.component_store::<DamageModelComponent>()?;
    let (player, _) = noita.get_player()?.context("no player")?;

    println!("hp: -");

    loop {
        let dmc = store.get(&player)?.context("no DMC")?;

        println!("\x1b[1F\x1b[2Jhp: {}", dmc.hp.get() * 25.0);

        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
#[ignore]
fn read_inventory() -> Result<()> {
    let mut noita = common::setup()?;

    let (entity, _) = noita.get_player()?.context("no player")?;

    let p = noita.proc().clone();

    let mut inv_quick = None;
    for child in entity.children.read(&p)?.read(&p)? {
        let child = child.read(&p)?;
        if child.name.read(&p)? == "inventory_quick" {
            inv_quick = Some(child);
            break;
        }
    }
    let inv_quick = inv_quick.context("Player has no inventory?")?;

    let store = noita.component_store::<ItemComponent>()?;
    let ac_store = noita.component_store::<AbilityComponent>()?;

    for child in inv_quick.children.read(&p)?.read(&p)? {
        let child = child.read(&p)?;

        if let Some(wand) = ac_store.get(&child)? {
            println!("{}", wand.sprite_file.read(&p)?);
        }

        let Some(item_comp) = store.get(&child)? else {
            continue;
        };
        println!("{:?}", item_comp.inventory_slot);
        let name = item_comp.item_name.read(&p)?;
        if name == "$booktitle_tree" {
            println!("best tablet detected");
        } else if name == "$item_evil_eye" {
            println!("evil eye detected");
        }
    }

    Ok(())
}

#[test]
#[ignore]
fn read_poly_pools() -> Result<()> {
    let noita = common::setup()?;

    let normal_pool = Ptr::<StdVec<StdString>>::of(0x012094dc);
    let rare_pool = Ptr::<StdVec<StdString>>::of(0x012219c8);

    let normal_pool = normal_pool.read(noita.proc())?.read_storage(noita.proc())?;
    let rare_pool = rare_pool.read(noita.proc())?.read_storage(noita.proc())?;

    println!("NORMAL:");
    for s in normal_pool {
        println!("  {s}")
    }

    println!("RARE:");
    for s in rare_pool {
        println!("  {s}")
    }

    Ok(())
}

#[test]
#[ignore]
fn read_perks() -> Result<()> {
    let mut noita = common::setup()?;

    let (entity, _) = noita.get_player()?.context("no player")?;

    let p = noita.proc().clone();

    let store = noita.component_store::<UIIconComponent>()?;

    let mut perks = HashMap::<_, usize>::new();

    for child in entity.children.read(&p)?.read(&p)? {
        let child = child.read(&p)?;
        let Some(ui) = store.get(&child)? else {
            continue;
        };
        let name = ui.name.read(&p)?;
        *perks.entry(name).or_default() += 1;
    }
    let mut perks = perks.into_iter().collect::<Vec<_>>();
    perks.sort_unstable_by_key(|(_, c)| -(*c as isize));

    for (perk, count) in perks {
        let perk = perk
            .strip_prefix("$perk_")
            .unwrap_or(&perk)
            .replace("_", " ");
        println!("{count}x {perk};");
    }

    Ok(())
}

#[test]
#[ignore]
fn seed_search() -> Result<()> {
    let (max_violations, max_seed) = (0..u32::MAX)
        .into_par_iter()
        .chunks(1_000_000)
        .map(|seeds| {
            let mut local_max_violations = 0;
            let mut local_max_seed = 0;

            for seed in seeds {
                let mut violations = 0;
                while NoitaRng::from_pos(seed, 64687.0, violations as _).in_range(1, 100) > 50 {
                    violations += 1;
                }
                if violations > local_max_violations && violations > 5 {
                    local_max_violations = violations;
                    local_max_seed = seed;
                }
            }
            // println!("local max violations: {local_max_violations}, seed {local_max_seed}");
            (local_max_violations, local_max_seed)
        })
        .max_by_key(|(v, _)| *v)
        .unwrap();

    println!("seed {max_seed} has {max_violations} consecutive violations");

    Ok(())
}

#[test]
#[ignore]
fn single_seed() -> Result<()> {
    let seed = 73691660;

    let mut res = String::new();

    for violation in 0..64 {
        let success = NoitaRng::from_pos(seed, 64687.0, violation as _).in_range(1, 100) > 50;
        res.push_str(if success { "1" } else { "0" });
    }
    println!("violations: {res}");

    Ok(())
}

#[test]
#[ignore]
fn ui_bitfield() -> Result<()> {
    let noita = common::setup()?;

    println!(
        "pause_flags: {:b}",
        noita.read_game_global()?.pause_flags.read(noita.proc())?
    );

    Ok(())
}

#[test]
#[ignore]
fn read_shifts() -> Result<()> {
    let mut noita = common::setup()?;

    let seed = noita.read_seed()?.context("no seed")?;
    println!("{seed}");

    let state = noita.get_world_state()?.context("no world state")?;
    let changed_materials = state.changed_materials.read_storage(noita.proc())?;

    let mut res: Vec<(Vec<String>, String)> = vec![];

    for (a, b) in changed_materials
        .iter()
        .step_by(2)
        .zip(changed_materials.iter().skip(1).step_by(2))
    {
        if let Some((from, to)) = res.last_mut() {
            if to == b {
                from.push(a.clone());
                continue;
            }
        }
        res.push((vec![a.clone()], b.clone()));
    }

    for (from, to) in res {
        println!("{} -> {to}", from.join(","));
    }

    // let mut lua_globals = state.lua_globals.read(noita.proc())?;
    // lua_globals.retain(|k, _| !k.starts_with("TEMPLE_ACTIVE_") && !k.starts_with("PERK_PICKED_"));
    // println!("{lua_globals:#?}");

    Ok(())
}

#[test]
#[ignore]
fn read_entities() -> Result<()> {
    let mut noita = common::setup()?;

    let (p, _) = noita.get_player()?.context("no player")?;

    let em = noita.read_entity_manager()?;
    let tags = noita.read_entity_tag_manager()?;
    let all_tags = tags.tags.read_storage(noita.proc())?;

    let entities = em.entities.read(noita.proc())?;
    println!("total: {}", entities.len());

    let lua_comp_store = noita.component_store::<LuaComponent>()?;

    let ctm = noita.read_component_type_manager()?;
    let component_names = ctm.component_indices.read(noita.proc())?;

    let mut idx_to_name = HashMap::new();
    for (k, v) in &component_names {
        idx_to_name.insert(*v, k.clone());
    }

    let comps = em.component_buffers.read(noita.proc())?;

    // let dist = 1000_f32;
    // let dist_sq = dist * dist;
    let mut count = 0;

    println!("player: {p:#?}");

    let mut nuggies = 0;

    for e in entities {
        if e.is_null() {
            continue;
        }
        let e = e.read(noita.proc())?;

        // let r_sq = (e.transform.pos.x - p.transform.pos.x).powi(2)
        //     + (e.transform.pos.y - p.transform.pos.y).powi(2);

        // if r_sq > dist_sq {
        //     continue;
        // }

        let mut tags = Vec::new();
        for (i, name) in all_tags.iter().enumerate() {
            if e.tags[i] {
                tags.push(name.clone());
            }
        }
        if tags.iter().any(|t| t == "gold_nugget") {
            nuggies += 1;
            continue;
        }
        if tags.iter().any(|t| {
            t == "perk_entity"
                || t == "card_action"
                || t == "wand"
                || t == "evil_eye"
                || t == "seed_b"
        }) {
            continue;
        }
        count += 1;
        let mut cnames = Vec::new();
        for (i, buf) in comps.iter().enumerate() {
            if buf.is_null() {
                continue;
            }
            let buf = buf.read(noita.proc())?;
            let idx = buf.indices.read_at(e.comp_idx, noita.proc())?;
            if let Some(idx) = idx {
                if idx != buf.default_index {
                    cnames.push(idx_to_name[&(i as _)].clone());
                }
            }
        }
        if let Some(lua_c) = lua_comp_store.get(&e)? {
            println!("lua: {lua_c:?}");
            println!("tags: {}", tags.join(", "));
            println!("{count}: {e:#?}");
            println!("components: {}", cnames.join(", "));
        }
    }

    println!("nuggies: {nuggies}");

    Ok(())
}

#[test]
#[ignore]
fn fs_things() -> Result<()> {
    let noita = common::setup()?;

    let platform = noita.read_platform()?;

    let test = platform.working_dir.read(noita.proc())?;
    println!("wd: {test}");

    let fs = platform.file_system.read(noita.proc())?;
    let devices = fs.devices.read(noita.proc())?;

    for device in devices {
        let Some(device) = FileDevice::get(noita.proc(), device)? else {
            continue;
        };
        println!("{device:#?}");
        // if let Some(file) = device.as_dyn().get_file(&self.proc, &fs, path)? {
        //     println!("found idk");
        // }
    }

    Ok(())
}

#[test]
#[ignore] // manual
fn process_disconnect() -> Result<()> {
    let noita = common::setup()?;

    println!("noita pid: {}", noita.proc().pid());
    std::thread::sleep(Duration::from_secs(10));

    let bounds = match noita.get_camera_bounds() {
        Err(e) if matches!(e.raw_os_error(), Some(3)) => {
            println!("noita.exe not connected lol");
            std::process::exit(1)
        }
        r => r?,
    };

    println!("bounds: {bounds:?}");

    Ok(())
}

#[test]
#[ignore]
fn materials_for_wuote() -> Result<()> {
    let noita = common::setup()?;
    let cf = noita.read_cell_factory()?.context("no cell factory")?;

    let materials = cf.material_ids.read_storage(noita.proc())?;
    let cell_data = cf.cell_data.read(noita.proc())?;

    let mut map = serde_json::Map::new();
    for (name, cell_data) in materials.into_iter().zip(cell_data.into_iter()) {
        map.insert(name, serde_json::to_value(cell_data)?);
    }

    println!("{}", serde_json::to_string_pretty(&map)?);

    Ok(())
}

#[test]
#[ignore] // manual
fn cell_reactions_for_wuote() -> Result<()> {
    let noita = common::setup()?;

    // let ws = Ptr::<Ptr<Component<WorldStateComponent>>>::of(0x01202ff0);
    // println!("{:#?}", { ws.read(&proc)?.read(&proc)?.data });

    // let player = noita.get_player()?.unwrap();
    // println!("{player:#?}");

    // let game_camera = noita.get_camera_pos();

    // println!("camera: {game_camera:?}");

    let cf = noita.read_cell_factory()?.context("no cell factory")?;

    let materials = cf.material_ids.read_storage(noita.proc())?;
    let reactions = cf.all_reactions(noita.proc())?;

    let name = |idx: i32| -> serde_json::Value {
        match idx {
            -1 => serde_json::Value::Null,
            _ => (&*materials[idx as usize]).into(),
        }
    };
    let entity_files: StdVec<StdString> = RawPtr::of(0x01207bd4).read(noita.proc())?;
    let entity_files = entity_files.read_storage(noita.proc())?;

    let entity_file = |idx: u32| -> serde_json::Value {
        match idx {
            0 => serde_json::Value::Null,
            _ => (&*entity_files[idx as usize]).into(),
        }
    };

    let mut res = vec![];

    for r in reactions {
        let explosion_config = if !r.explosion_config.is_null() {
            let e = r.explosion_config.read(noita.proc())?;
            Some(serde_json::json! {{
              "never_cache": e.never_cache.get().as_bool(),
              "explosion_radius": e.explosion_radius,
              "explosion_sprite": e.explosion_sprite.read(noita.proc())?,
              "explosion_sprite_emissive": e.explosion_sprite_emissive.as_bool(),
              "explosion_sprite_additive": e.explosion_sprite_additive.as_bool(),
              "explosion_sprite_random_rotation": e.explosion_sprite_random_rotation.get().as_bool(),
              "explosion_sprite_lifetime": e.explosion_sprite_lifetime,
              "damage": e.damage,
              "damage_critical": {
                  "chance": e.damage_critical.chance,
                  "damage_multiplier": e.damage_critical.damage_multiplier,
                  "m_succeeded": e.damage_critical.m_succeeded.get().as_bool(),
              },
              "camera_shake": e.camera_shake,
              "particle_effect": e.particle_effect.get().as_bool(),
              "load_this_entity": e.load_this_entity.read(noita.proc())?,
              "light_enabled": e.light_enabled.get().as_bool(),
              "light_fade_time": e.light_fade_time,
              "light_r": e.light_r,
              "light_g": e.light_g,
              "light_b": e.light_b,
              "light_radius_coeff": e.light_radius_coeff,
              "hole_enabled": e.hole_enabled.as_bool(),
              "destroy_non_platform_solid_enabled": e.destroy_non_platform_solid_enabled.get().as_bool(),
              "electricity_count": e.electricity_count,
              "min_radius_for_cracks": e.min_radius_for_cracks,
              "crack_count": e.crack_count,
              "knockback_force": e.knockback_force,
              "hole_destroy_liquid": e.hole_destroy_liquid.as_bool(),
              "hole_destroy_physics_dynamic": e.hole_destroy_physics_dynamic.get().as_bool(),
              "create_cell_material": e.create_cell_material.read(noita.proc())?,
              "create_cell_probability": e.create_cell_probability,
              "background_lightning_count": e.background_lightning_count,
              "spark_material": e.spark_material.read(noita.proc())?,
              "material_sparks_min_hp": e.material_sparks_min_hp,
              "material_sparks_probability": e.material_sparks_probability,
              "material_sparks_count": {
                  "min": e.material_sparks_count.min,
                  "max": e.material_sparks_count.max,
              },
              "material_sparks_enabled": e.material_sparks_enabled.as_bool(),
              "material_sparks_real": e.material_sparks_real.as_bool(),
              "material_sparks_scale_with_hp": e.material_sparks_scale_with_hp.as_bool(),
              "sparks_enabled": e.sparks_enabled.as_bool(),
              "sparks_count": {
                  "min": e.sparks_count.min,
                  "max": e.sparks_count.max,
              },
              "sparks_inner_radius_coeff": e.sparks_inner_radius_coeff,
              "stains_enabled": e.stains_enabled.get().as_bool(),
              "stains_radius": e.stains_radius,
              "ray_energy": e.ray_energy,
              "max_durability_to_destroy": e.max_durability_to_destroy,
              "gore_particle_count": e.gore_particle_count,
              "shake_vegetation": e.shake_vegetation.as_bool(),
              "damage_mortals": e.damage_mortals.as_bool(),
              "physics_throw_enabled": e.physics_throw_enabled.get().as_bool(),
              "physics_explosion_power": {
                  "min": e.physics_explosion_power.min,
                  "max": e.physics_explosion_power.max,
              },
              "physics_multiplier_ragdoll_force": e.physics_multiplier_ragdoll_force,
              "cell_explosion_power": e.cell_explosion_power,
              "cell_explosion_radius_min": e.cell_explosion_radius_min,
              "cell_explosion_radius_max": e.cell_explosion_radius_max,
              "cell_explosion_velocity_min": e.cell_explosion_velocity_min,
              "cell_explosion_damage_required": e.cell_explosion_damage_required,
              "cell_explosion_probability": e.cell_explosion_probability,
              "cell_power_ragdoll_coeff": e.cell_power_ragdoll_coeff,
              "pixel_sprites_enabled": e.pixel_sprites_enabled.as_bool(),
              "is_digger": e.is_digger.as_bool(),
              "audio_enabled": e.audio_enabled.get().as_bool(),
              "audio_event_name": e.audio_event_name.read(noita.proc())?,
              "audio_liquid_amount_normalized": e.audio_liquid_amount_normalized,
              "delay": {
                  "min": e.delay.min,
                  "max": e.delay.max,
              },
              "explosion_delay_id": e.explosion_delay_id,
              "not_scaled_by_gamefx": e.not_scaled_by_gamefx.get().as_bool(),
              "who_is_responsible": e.who_is_responsible,
              "null_damage": e.null_damage.get().as_bool(),
              "dont_damage_this": e.dont_damage_this,
              "impl_send_message_to_this": e.impl_send_message_to_this,
              "impl_position": {
                  "x": e.impl_position.x,
                  "y": e.impl_position.y,
              },
              "impl_delay_frame": e.impl_delay_frame,
            }})
        } else {
            None
        };

        let json = serde_json::json! {{
            "fast_reaction": r.fast_reaction.get().as_bool(),
            "probability_times_100": r.probability_times_100,
            "input_cell1": name(r.input_cell1),
            "input_cell2": name(r.input_cell2),
            "output_cell1": name(r.output_cell1),
            "output_cell2": name(r.output_cell2),
            "has_input_cell3": r.has_input_cell3.get().as_bool(),
            "input_cell3": name(r.input_cell3),
            "output_cell3": name(r.output_cell3),
            "cosmetic_particle": name(r.cosmetic_particle),
            "req_lifetime": r.req_lifetime,
            "blob_radius1": r.blob_radius1,
            "blob_radius2": r.blob_radius2,
            "blob_restrict_to_input_material1": r.blob_restrict_to_input_material1.as_bool(),
            "blob_restrict_to_input_material2": r.blob_restrict_to_input_material2.as_bool(),
            "destroy_horizontally_lonely_pixels": r.destroy_horizontally_lonely_pixels.as_bool(),
            "convert_all": r.convert_all.get().as_bool(),
            "entity_file": entity_file(r.entity_file_idx),
            "direction": format!("{:?}", r.direction).to_lowercase(),
            "explosion_config": explosion_config,
            "audio_fx_volume_1": r.audio_fx_volume_1,
        }};
        res.push(json);
    }

    std::fs::write("cell_reactions.json", serde_json::to_string_pretty(&res)?)?;

    Ok(())
}
