pub use noita_engine_reader_macros::ComponentName;
use open_enum::open_enum;
use serde::Serialize;
use zerocopy::{FromBytes, IntoBytes};

use crate::memory::{
    Align4, ByteBool, CString, PadBool, Pod, Ptr, PtrReadable, StdMap, StdString, StdVec, Vftable,
    WithPad,
};

use super::{Bitset256, Entity, Vec2, Vec2i};

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C, packed)]
pub struct Component<D> {
    pub vftable: Vftable,
    _field_0x4: u32,
    pub type_name: CString,
    pub type_id: u32,
    pub instance_id: u32,
    pub enabled: PadBool<3>,
    pub tags: Bitset256,
    some_vec: StdVec<u32>, // no idea what this is yet,
    _field_0x44: u32,
    pub data: D,
}

impl<D: Pod> PtrReadable for Component<D> {}

pub trait ComponentName {
    const NAME: &str;
}

#[derive(ComponentName, FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct WalletComponent {
    pub money: Align4<u64>,
    pub money_spent: Align4<u64>,
    pub m_money_prev_frame: Align4<u64>,
    pub m_has_reached_inf: PadBool<3>,
}

#[derive(ComponentName, FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ItemComponent {
    pub item_name: StdString,
    pub is_stackable: ByteBool,
    pub is_consumable: ByteBool,
    pub stats_count_as_item_pick_up: ByteBool,
    pub auto_pickup: ByteBool,
    pub permanently_attached: PadBool<3>,
    pub uses_remaining: i32,
    pub is_identified: ByteBool,
    pub is_frozen: ByteBool,
    pub collect_nondefault_actions: ByteBool,
    pub remove_on_death: ByteBool,
    pub remove_on_death_if_empty: ByteBool,
    pub remove_default_child_actions_on_death: ByteBool,
    pub play_hover_animation: ByteBool,
    pub play_spinning_animation: ByteBool,
    pub is_equipable_forced: ByteBool,
    pub play_pick_sound: ByteBool,
    pub drinkable: PadBool<1>,
    pub spawn_pos: Vec2,
    pub max_child_items: i32,
    pub ui_sprite: StdString,
    pub ui_description: StdString,
    pub preferred_inventory: u32,
    pub enable_orb_hacks: u8,
    pub is_all_spells_book: u8,
    pub always_use_item_name_in_ui: PadBool<1>,
    pub custom_pickup_string: StdString,
    pub ui_display_description_on_pick_up_hint: PadBool<3>,
    pub inventory_slot: Vec2i,
    pub next_frame_pickable: i32,
    pub npc_next_frame_pickable: i32,
    pub is_pickable: ByteBool,
    pub is_hittable_always: PadBool<2>,
    pub item_pickup_radius: f32,
    pub camera_max_distance: f32,
    pub camera_smooth_speed_multiplier: f32,
    pub has_been_picked_by_player: PadBool<3>,
    pub m_frame_picked_up: i32,
    pub m_item_uid: i32,
    pub m_is_identified: PadBool<3>,
}

#[derive(ComponentName, FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ItemActionComponent {
    pub action_id: StdString,
}

#[derive(ComponentName, FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct MaterialInventoryComponent {
    pub drop_as_item: ByteBool,
    pub on_death_spill: ByteBool,
    pub leak_gently: PadBool<1>,
    pub leak_on_damage_percent: f32,
    pub leak_pressure_min: f32,
    pub leak_pressure_max: f32,
    pub min_damage_to_leak: f32,
    pub b2_force_on_leak: f32,
    pub death_throw_particle_velocity_coeff: f32,
    pub kill_when_empty: ByteBool,
    pub halftime_materials: PadBool<2>,
    pub do_reactions: i32,
    pub do_reactions_explosions: ByteBool,
    pub do_reactions_entities: PadBool<2>,
    pub reaction_speed: i32,
    pub reactions_shaking_speeds_up: PadBool<3>,
    pub max_capacity: Align4<f64>,
    pub count_per_material_type: StdVec<f64>,
    pub audio_collision_size_modifier_amount: f32,
    pub is_death_handled: PadBool<3>,
    pub last_frame_drank: i32,
    pub ex_position: Vec2,
    pub ex_angle: f32,
}

#[derive(ComponentName, FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct DamageModelComponent {
    pub hp: Align4<f64>,
    pub max_hp: Align4<f64>,
    pub max_hp_cap: Align4<f64>,
    pub max_hp_old: Align4<f64>,
    pub damage_multipliers: ConfigDamagesByType,
    pub critical_damage_resistance: f32,
    pub invincibility_frames: i32,
    pub falling_damages: PadBool<3>,
    pub falling_damage_height_min: f32,
    pub falling_damage_height_max: f32,
    pub falling_damage_damage_min: f32,
    pub falling_damage_damage_max: f32,
    pub air_needed: PadBool<3>,
    pub air_in_lungs: f32,
    pub air_in_lungs_max: f32,
    pub air_lack_of_damage: f32,
    pub minimum_knockback_force: f32,
    pub materials_damage: PadBool<3>,
    pub material_damage_min_cell_count: i32,
    pub materials_that_damage: StdString,
    pub materials_how_much_damage: StdString,
    pub materials_damage_proportional_to_maxhp: ByteBool,
    pub physics_objects_damage: ByteBool,
    pub materials_create_messages: PadBool<1>,
    pub materials_that_create_messages: StdString,
    pub ragdoll_filenames_file: StdString,
    pub ragdoll_material: StdString,
    pub ragdoll_offset_x: f32,
    pub ragdoll_offset_y: f32,
    pub ragdoll_fx_forced: i32, // enum
    pub blood_material: StdString,
    pub blood_spray_material: StdString,
    pub blood_spray_create_some_cosmetic: PadBool<3>,
    pub blood_multiplier: f32,
    pub ragdoll_blood_amount_absolute: i32,
    pub blood_sprite_directional: StdString,
    pub blood_sprite_large: StdString,
    pub healing_particle_effect_entity: StdString,
    pub create_ragdoll: ByteBool,
    pub ragdollify_child_entity_sprites: PadBool<2>,
    pub ragdollify_root_angular_damping: f32,
    pub ragdollify_disintegrate_nonroot: ByteBool,
    pub wait_for_kill_flag_on_death: ByteBool,
    pub kill_now: ByteBool,
    pub drop_items_on_death: ByteBool,
    pub ui_report_damage: ByteBool,
    pub ui_force_report_damage: PadBool<2>,
    pub in_liquid_shooting_electrify_prob: i32,
    pub wet_status_effect_damage: f32,
    pub is_on_fire: PadBool<3>,
    pub fire_probability_of_ignition: f32,
    pub fire_how_much_fire_generates: i32,
    pub fire_damage_ignited_amount: f32,
    pub fire_damage_amount: f32,
    pub m_is_on_fire: PadBool<3>,
    pub m_fire_probability: i32,
    pub m_fire_frames_left: i32,
    pub m_fire_duration_frames: i32,
    pub m_fire_tried_igniting: PadBool<3>,
    pub m_last_check_x: i32,
    pub m_last_check_y: i32,
    pub m_last_check_time: i32,
    pub m_last_material_damage_frame: i32,
    pub m_fall_is_on_ground: PadBool<3>,
    pub m_fall_highest_y: f32,
    pub m_fall_count: i32,
    pub m_air_are_we_in_water: PadBool<3>,
    pub m_air_frames_not_in_water: i32,
    pub m_air_do_we_have: PadBool<3>,
    pub m_total_cells: i32,
    pub m_liquid_count: i32,
    pub m_liquid_material_we_are_in: i32,
    pub m_damage_materials: StdVec<i32>,
    pub m_damage_materials_how_much: StdVec<f32>,
    pub m_collision_message_materials: StdVec<i32>,
    pub m_collision_message_material_counts_this_frame: StdVec<i32>,
    pub m_material_damage_this_frame: StdVec<f32>,
    pub m_fall_damage_this_frame: f32,
    pub m_electricity_damage_this_frame: f32,
    pub m_physics_damage_this_frame: f32,
    pub m_physics_damage_vec_this_frame: Vec2,
    pub m_physics_damage_last_frame: i32,
    pub m_physics_damage_entity: u32,
    pub m_physics_damage_telekinesis_caster_entity: u32,
    pub m_last_damage_frame: i32,
    pub m_hp_before_last_damage: Align4<f64>,
    pub m_last_electricity_resistance_frame: i32,
    pub m_last_frame_reported_block: i32,
    pub m_last_max_hp_change_frame: i32,
    pub m_fire_damage_buffered: f32,
    pub m_fire_damage_buffered_next_delivery_frame: i32,
}
const _: () = assert!(std::mem::size_of::<DamageModelComponent>() == 0x294);

#[derive(ComponentName, FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct UIIconComponent {
    pub icon_sprite_file: StdString,
    pub name: StdString,
    pub description: StdString,
    pub display_above_head: ByteBool,
    pub display_in_hud: ByteBool,
    pub is_perk: PadBool<1>,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ConfigGun {
    pub vftable: Vftable,
    pub actions_per_round: i32,
    pub shuffle_deck_when_empty: PadBool<3>,
    pub reload_time: i32,
    pub deck_capacity: i32,
}
const _: () = assert!(std::mem::size_of::<ConfigGun>() == 0x14);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ConfigGunActionInfo {
    pub vftable: Vftable,
    pub action_id: StdString,
    pub action_name: StdString,
    pub action_description: StdString,
    pub action_sprite_filename: StdString,
    pub action_unidentified_sprite_filename: StdString,
    pub action_type: i32,
    pub action_spawn_level: StdString,
    pub action_spawn_probability: StdString,
    pub action_spawn_requires_flag: StdString,
    pub action_spawn_manual_unlock: PadBool<3>,
    pub action_max_uses: i32,
    pub custom_xml_file: StdString,
    pub action_mana_drain: f32,
    pub action_is_dangerous_blast: PadBool<3>,
    pub action_draw_many_count: i32,
    pub action_ai_never_uses: ByteBool,
    pub action_never_unlimited: ByteBool,
    pub state_shuffled: PadBool<1>,
    pub state_cards_drawn: i32,
    pub state_discarded_action: ByteBool,
    pub state_destroyed_action: PadBool<2>,
    pub fire_rate_wait: i32,
    pub speed_multiplier: f32,
    pub child_speed_multiplier: f32,
    pub dampening: f32,
    pub explosion_radius: f32,
    pub spread_degrees: f32,
    pub pattern_degrees: f32,
    pub screenshake: f32,
    pub recoil: f32,
    pub damage_melee_add: f32,
    pub damage_projectile_add: f32,
    pub damage_electricity_add: f32,
    pub damage_fire_add: f32,
    pub damage_explosion_add: f32,
    pub damage_ice_add: f32,
    pub damage_slice_add: f32,
    pub damage_healing_add: f32,
    pub damage_curse_add: f32,
    pub damage_drill_add: f32,
    pub damage_null_all: f32,
    pub damage_critical_chance: i32,
    pub damage_critical_multiplier: f32,
    pub explosion_damage_to_materials: f32,
    pub knockback_force: f32,
    pub reload_time: i32,
    pub lightning_count: i32,
    pub material: StdString,
    pub material_amount: i32,
    pub trail_material: StdString,
    pub trail_material_amount: i32,
    pub bounces: i32,
    pub gravity: f32,
    pub light: f32,
    pub blood_count_multiplier: f32,
    pub gore_particles: i32,
    pub ragdoll_fx: i32,
    pub friendly_fire: PadBool<3>,
    pub physics_impulse_coeff: f32,
    pub lifetime_add: i32,
    pub sprite: StdString,
    pub extra_entities: StdString,
    pub game_effect_entities: StdString,
    pub sound_loop_tag: StdString,
    pub projectile_file: StdString,
}
const _: () = assert!(std::mem::size_of::<ConfigGunActionInfo>() == 0x23c);

#[derive(ComponentName, FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct AbilityComponent {
    pub cooldown_frames: i32,
    pub entity_file: StdString,
    pub sprite_file: StdString,
    pub entity_count: i32,
    pub never_reload: PadBool<3>,
    pub reload_time_frames: i32,
    pub mana: f32,
    pub mana_max: f32,
    pub mana_charge_speed: f32,
    pub rotate_in_hand: PadBool<3>,
    pub rotate_in_hand_amount: f32,
    pub rotate_hand_amount: f32,
    pub fast_projectile: PadBool<3>,
    pub swim_propel_amount: f32,
    pub max_charged_actions: i32,
    pub charge_wait_frames: i32,
    pub item_recoil_recovery_speed: f32,
    pub item_recoil_max: f32,
    pub item_recoil_offset_coeff: f32,
    pub item_recoil_rotation_coeff: f32,
    pub base_item_file: StdString,
    pub use_entity_file_as_projectile_info_proxy: ByteBool,
    pub click_to_use: PadBool<2>,
    pub stat_times_player_has_shot: i32,
    pub stat_times_player_has_edited: i32,
    pub shooting_reduces_amount_in_inventory: ByteBool,
    pub throw_as_item: ByteBool,
    pub simulate_throw_as_item: PadBool<1>,
    pub max_amount_in_inventory: i32,
    pub amount_in_inventory: i32,
    pub drop_as_item_on_death: PadBool<3>,
    pub ui_name: StdString,
    pub use_gun_script: ByteBool,
    pub is_petris_gun: PadBool<2>,
    pub gun_config: ConfigGun,
    pub gunaction_config: ConfigGunActionInfo,
    pub gun_level: i32,
    pub add_these_child_actions: StdString,
    pub current_slot_durability: i32,
    pub slot_consumption_function: StdString,
    pub m_next_frame_usable: i32,
    pub m_cast_delay_start_frame: i32,
    pub m_ammo_left: i32,
    pub m_reload_frames_left: i32,
    pub m_reload_next_frame_usable: i32,
    pub m_charge_count: i32,
    pub m_next_charge_frame: i32,
    pub m_item_recoil: f32,
    pub m_is_initialized: PadBool<3>,
}
const _: () = assert!(std::mem::size_of::<AbilityComponent>() == 0x374);

#[derive(FromBytes, IntoBytes, Debug, Serialize)]
#[repr(C)]
pub struct ConfigDamagesByType {
    #[serde(skip)]
    pub vftable: Vftable,
    pub melee: f32,
    pub projectile: f32,
    pub explosion: f32,
    pub electricity: f32,
    pub fire: f32,
    pub drill: f32,
    pub slice: f32,
    pub ice: f32,
    pub healing: f32,
    pub physics_hit: f32,
    pub radioactive: f32,
    pub poison: f32,
    pub overeating: f32,
    pub curse: f32,
    pub holy: f32,
}
const _: () = assert!(std::mem::size_of::<ConfigDamagesByType>() == 0x40);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct LensValueBool {
    pub value: WithPad<ByteBool, 3>,
    pub unknown: i32,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C, packed)]
pub struct LensValue<T> {
    pub value: T,
    pub _unknown2: u32,
    pub unknown: i32,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ConfigPendingPortal {
    pub vftable: Vftable,
    pub position: Vec2,
    pub target_position: Vec2,
    pub id: u32,
    pub target_id: u32,
    pub is_at_home: WithPad<ByteBool, 3>,
    pub target_biome_name: StdString,
    pub entity: Ptr<Entity>,
}
const _: () = assert!(std::mem::size_of::<ConfigPendingPortal>() == 0x3c);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ConfigNpcParty {
    pub vftable: Vftable,
    pub position: Vec2,
    pub entities_exist: WithPad<ByteBool, 3>,
    pub direction: i32,
    pub speed: f32,
    pub member_entities: StdVec<u32>,
    pub member_files: StdVec<StdString>,
}
const _: () = assert!(std::mem::size_of::<ConfigNpcParty>() == 0x30);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ConfigCutThroughWorld {
    pub vftable: Vftable,
    pub x: i32,
    pub y_min: i32,
    pub y_max: i32,
    pub radius: i32,
    pub edge_darkening_width: i32,
    pub global_id: u32,
}
const _: () = assert!(std::mem::size_of::<ConfigCutThroughWorld>() == 0x1c);

#[derive(ComponentName, FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct WorldStateComponent {
    pub is_initialized: WithPad<ByteBool, 3>,
    pub time: f32,
    pub time_total: f32,
    pub time_dt: f32,
    pub day_count: i32,
    pub rain: f32,
    pub rain_target: f32,
    pub fog: f32,
    pub fog_target: f32,
    pub intro_weather: WithPad<ByteBool, 3>,
    pub wind: f32,
    pub wind_speed: f32,
    pub wind_speed_sin_t: f32,
    pub wind_speed_sin: f32,
    pub clouds_01_target: f32,
    pub clouds_02_target: f32,
    pub gradient_sky_alpha_target: f32,
    pub sky_sunset_alpha_target: f32,
    pub lightning_count: i32,
    pub player_spawn_location: Vec2,
    pub lua_globals: StdMap<StdString, StdString>,
    pub pending_portals: StdVec<ConfigPendingPortal>,
    pub next_portal_id: u32,
    pub apparitions_per_level: StdVec<i32>,
    pub npc_parties: StdVec<ConfigNpcParty>,
    pub session_stat_file: StdString,
    pub orbs_found_thisrun: StdVec<i32>,
    pub flags: StdVec<StdString>,
    pub changed_materials: StdVec<StdString>,
    pub player_polymorph_count: i32,
    pub player_polymorph_random_count: i32,
    pub player_did_infinite_spell_count: i32,
    pub player_did_damage_over_1milj: i32,
    pub player_living_with_minus_hp: i32,
    pub global_genome_relations_modifier: f32,
    pub mods_have_been_active_during_this_run: ByteBool,
    pub twitch_has_been_active_during_this_run: WithPad<ByteBool, 2>,
    pub next_cut_through_world_id: u32,
    pub cuts_through_world: StdVec<ConfigCutThroughWorld>,
    pub gore_multiplier: LensValue<i32>,
    pub trick_kill_gold_multiplier: LensValue<i32>,
    pub damage_flash_multiplier: LensValue<f32>,
    pub open_fog_of_war_everywhere: LensValueBool,
    pub consume_actions: LensValueBool,
    pub perk_infinite_spells: ByteBool,
    pub perk_trick_kills_blood_money: WithPad<ByteBool, 2>,
    pub perk_hp_drop_chance: i32,
    pub perk_gold_is_forever: ByteBool,
    pub perk_rats_player_friendly: ByteBool,
    pub everything_to_gold: WithPad<ByteBool, 1>,
    pub material_everything_to_gold: StdString,
    pub material_everything_to_gold_static: StdString,
    pub infinite_gold_happening: ByteBool,
    pub ending_happiness_happening: WithPad<ByteBool, 2>,
    pub ending_happiness_frames: i32,
    pub ending_happiness: WithPad<ByteBool, 3>,
    pub m_flash_alpha: f32,
    pub debug_loaded_from_autosave: i32,
    pub debug_loaded_from_old_version: i32,
    pub rain_target_extra: f32,
    pub fog_target_extra: f32,
    pub perk_rats_player_friendly_prev: WithPad<ByteBool, 3>,
}
const _: () = assert!(std::mem::size_of::<WorldStateComponent>() == 0x180);

#[open_enum]
#[repr(u32)]
#[derive(FromBytes, IntoBytes, Debug)]
pub enum LuaVmType {
    SharedByManyComponents,
    CreateNewEveryExecution,
    OnePerComponentInstance,
}

#[derive(ComponentName, FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct LuaComponent {
    pub script_source_file: StdString,
    pub vm_type: LuaVmType,
    pub execute_on_added: ByteBool,
    pub execute_on_removed: PadBool<2>,
    pub execute_every_n_frame: i32,
    pub execute_times: i32,
    pub limit_how_many_times_per_frame: i32,
    pub limit_to_every_n_frame: i32,
    pub limit_all_callbacks: ByteBool,
    pub remove_after_executed: ByteBool,
    pub enable_coroutines: ByteBool,
    pub call_init_function: ByteBool,
    pub script_enabled_changed: StdString,
    pub script_damage_received: StdString,
    pub script_damage_about_to_be_received: StdString,
    pub script_item_picked_up: StdString,
    pub script_shot: StdString,
    pub script_collision_trigger_hit: StdString,
    pub script_collision_trigger_timer_finished: StdString,
    pub script_physics_body_modified: StdString,
    pub script_pressure_plate_change: StdString,
    pub script_inhaled_material: StdString,
    pub script_death: StdString,
    pub script_throw_item: StdString,
    pub script_material_area_checker_failed: StdString,
    pub script_material_area_checker_success: StdString,
    pub script_electricity_receiver_switched: StdString,
    pub script_electricity_receiver_electrified: StdString,
    pub script_kick: StdString,
    pub script_interacting: StdString,
    pub script_audio_event_dead: StdString,
    pub script_wand_fired: StdString,
    pub script_teleported: StdString,
    pub script_portal_teleport_used: StdString,
    pub script_polymorphing_to: StdString,
    pub script_biome_entered: StdString,
    pub m_last_execution_frame: i32,
    pub m_times_executed_this_frame: i32,
    pub m_mod_appends_done: PadBool<3>,
    pub m_next_execution_time: i32,
    pub m_times_executed: i32,
    pub m_lua_manager: u32, // undefined
    pub m_persistent_values: i32,
}

const _: () = assert!(std::mem::size_of::<LuaComponent>() == 0x2d8 - 0x48);

#[open_enum]
#[repr(u32)]
#[derive(FromBytes, IntoBytes, Debug, Clone, Copy)]
pub enum GameEffect {
    None = 0,
    Electrocution = 1,
    Frozen = 2,
    OnFire = 3,
    Poison = 4,
    Berserk = 5,
    Charm = 6,
    Polymorph = 7,
    PolymorphRandom = 8,
    Blindness = 9,
    Telepathy = 10,
    Teleportation = 11,
    Regeneration = 12,
    Levitation = 13,
    MovementSlower = 14,
    Farts = 15,
    Drunk = 16,
    BreathUnderwater = 19,
    Radioactive = 20,
    Wet = 21,
    Oiled = 22,
    Bloody = 23,
    Slimy = 24,
    CriticalHitBoost = 25,
    Confusion = 26,
    MeleeCounter = 27,
    WormAttractor = 28,
    WormDetractor = 29,
    FoodPoisoning = 30,
    FriendThundermage = 31,
    FriendFiremage = 32,
    InternalFire = 33,
    InternalIce = 34,
    Jarate = 35,
    Knockback = 36,
    KnockbackImmunity = 37,
    MovementSlower2X = 38,
    MovementFaster = 40,
    StainsDropFaster = 41,
    SavingGrace = 42,
    DamageMultiplier = 43,
    HealingBlood = 44,
    Respawn = 45,
    ProtectionFire = 46,
    ProtectionRadioactivity = 47,
    ProtectionExplosion = 48,
    ProtectionMelee = 49,
    ProtectionElectricity = 50,
    Teleportitis = 51,
    StainlessArmour = 52,
    GlobalGore = 53,
    EditWandsEverywhere = 54,
    ExplodingCorpseShots = 55,
    ExplodingCorpse = 56,
    ExtraMoney = 57,
    ExtraMoneyTrickKill = 58,
    HoverBoost = 60,
    ProjectileHoming = 61,
    AbilityActionsMaterialized = 62,
    NoDamageFlash = 70,
    NoSlimeSlowdown = 71,
    MovementFaster2X = 72,
    NoWandEditing = 73,
    LowHpDamageBoost = 74,
    FasterLevitation = 75,
    StunProtectionElectricity = 76,
    StunProtectionFreeze = 77,
    IronStomach = 78,
    ProtectionAll = 80,
    Invisibility = 81,
    RemoveFogOfWar = 82,
    ManaRegeneration = 83,
    ProtectionDuringTeleport = 84,
    ProtectionPolymorph = 85,
    ProtectionFreeze = 86,
    FrozenSpeedUp = 87,
    UnstableTeleportation = 88,
    PolymorphUnstable = 89,
    Custom = 90,
    AllergyRadioactive = 91,
    RainbowFarts = 92,
    Weakness = 93,
    ProtectionFoodPoisoning = 94,
    NoHeal = 95,
    ProtectionEdges = 96,
    ProtectionProjectile = 97,
    PolymorphCessation = 98,
    _Last = 99,
}

#[derive(ComponentName, FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct GameEffectComponent {
    pub effect: GameEffect,
    pub custom_effect_id: StdString,
    pub frames: i32,
    pub exclusivity_group: i32,
    pub report_block_msg: ByteBool,
    pub disable_movement: PadBool<2>,
    pub ragdoll_effect: i32,
    pub ragdoll_material: i32,
    pub ragdoll_effect_custom_entity_file: StdString,
    pub ragdoll_fx_custom_entity_apply_only_to_largest_body: PadBool<3>,
    pub polymorph_target: StdString,
    pub m_serialized_data: StdString,
    pub m_caster: u32,
    pub m_caster_herd_id: i32,
    pub teleportation_probability: i32,
    pub teleportation_delay_min_frames: i32,
    pub teleportation_radius_min: f32,
    pub teleportation_radius_max: f32,
    pub teleportations_num: i32,
    pub no_heal_max_hp_cap: Align4<f64>,
    pub causing_status_effect: u32,
    pub caused_by_ingestion_status_effect: ByteBool,
    pub caused_by_stains: ByteBool,
    pub m_charm_disabled_camera_bound: ByteBool,
    pub m_charm_enabled_teleporting: ByteBool,
    pub m_invisible: PadBool<3>,
    pub m_counter: i32,
    pub m_cooldown: i32,
    pub m_is_extension: ByteBool,
    pub m_is_spent: PadBool<2>,
}
const _: () = assert!(std::mem::size_of::<GameEffectComponent>() == 0xb8);

#[derive(ComponentName, FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct PotionComponent {
    pub spray_velocity_coeff: f32,
    pub spray_velocity_normalized_min: f32,
    pub body_colored: ByteBool,
    pub throw_bunch: PadBool<2>,
    pub throw_how_many: i32,
    pub dont_spray_static_materials: ByteBool,
    pub dont_spray_just_leak_gas_materials: ByteBool,
    pub never_color: PadBool<1>,
    pub custom_color_material: i32,
}
const _: () = assert!(std::mem::size_of::<PotionComponent>() == 0x18);
