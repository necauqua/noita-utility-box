use zerocopy::{FromBytes, IntoBytes};

use crate::memory::{Align4, ByteBool, CString, PadBool, StdString, StdVec, Vftable};

use super::{Bitset256, Vec2, Vec2i};

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

pub trait ComponentName {
    const NAME: &str;
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct WalletComponent {
    pub money: Align4<u64>,
    pub money_spent: Align4<u64>,
    pub m_money_prev_frame: Align4<u64>,
    pub m_has_reached_inf: PadBool<3>,
}

impl ComponentName for WalletComponent {
    const NAME: &str = "WalletComponent";
}

#[derive(FromBytes, IntoBytes, Debug)]
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

impl ComponentName for ItemComponent {
    const NAME: &str = "ItemComponent";
}

#[derive(FromBytes, IntoBytes, Debug)]
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

impl ComponentName for MaterialInventoryComponent {
    const NAME: &str = "MaterialInventoryComponent";
}

#[derive(FromBytes, IntoBytes, Debug)]
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

impl ComponentName for DamageModelComponent {
    const NAME: &str = "DamageModelComponent";
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ConfigDamagesByType {
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
