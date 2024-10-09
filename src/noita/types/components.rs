use zerocopy::{AsBytes, FromBytes, FromZeroes};

use crate::memory::{CString, PadBool, RawPtr, RealignedF64, StdString, StdVec};

use super::{Bitset256, Vec2, Vec2i};

#[derive(AsBytes, FromBytes, FromZeroes, Debug)]
#[repr(C)]
pub struct Component {
    pub vftable: RawPtr,
    _field_0x4: u32,
    pub type_name: CString,
    pub type_id: u32,
    pub instance_id: u32,
    pub enabled: PadBool<3>,
    pub tags: Bitset256,
    some_vec: StdVec<u32>, // no idea what this is yet,
    _field_0x44: u32,
}

pub trait ComponentName {
    const NAME: &str;
}

#[derive(AsBytes, FromBytes, FromZeroes, Debug)]
#[repr(C)]
pub struct WalletComponent {
    pub parent: Component,
    pub money: u64,
    pub money_spent: u64,
    pub money_prev_frame: u64,
    pub money_infinite: PadBool<7>,
}

impl ComponentName for WalletComponent {
    const NAME: &str = "WalletComponent";
}

#[derive(AsBytes, FromBytes, FromZeroes, Debug)]
#[repr(C)]
pub struct ItemComponent {
    pub parent: Component,
    pub item_name: StdString,
    pub is_stackable: PadBool,
    pub is_consumable: PadBool,
    pub stats_count_as_item_pick_up: PadBool,
    pub auto_pickup: PadBool,
    pub permanently_attached: PadBool<3>,
    pub uses_remaining: i32,
    pub is_identified: PadBool,
    pub is_frozen: PadBool,
    pub collect_nondefault_actions: PadBool,
    pub remove_on_death: PadBool,
    pub remove_on_death_if_empty: PadBool,
    pub remove_default_child_actions_on_death: PadBool,
    pub play_hover_animation: PadBool,
    pub play_spinning_animation: PadBool,
    pub is_equipable_forced: PadBool,
    pub play_pick_sound: PadBool,
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
    pub is_pickable: PadBool,
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

#[derive(AsBytes, FromBytes, FromZeroes, Debug)]
#[repr(C)]
pub struct MaterialInventoryComponent {
    pub parent: Component,
    pub drop_as_item: PadBool,
    pub on_death_spill: PadBool,
    pub leak_gently: PadBool<1>,
    pub leak_on_damage_percent: f32,
    pub leak_pressure_min: f32,
    pub leak_pressure_max: f32,
    pub min_damage_to_leak: f32,
    pub b2_force_on_leak: f32,
    pub death_throw_particle_velocity_coeff: f32,
    pub kill_when_empty: PadBool,
    pub halftime_materials: PadBool<2>,
    pub do_reactions: i32,
    pub do_reactions_explosions: PadBool,
    pub do_reactions_entities: PadBool<2>,
    pub reaction_speed: i32,
    pub reactions_shaking_speeds_up: PadBool<3>,
    pub max_capacity: RealignedF64,
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
