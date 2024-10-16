use zerocopy::{FromBytes, IntoBytes};

use crate::memory::{ByteBool, CString, PadBool, RawPtr, StdString, StdVec};

use super::{Bitset256, Vec2, Vec2i};

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C, packed)]
pub struct Component<D> {
    pub vftable: RawPtr,
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
    pub money: u64,
    pub money_spent: u64,
    pub money_prev_frame: u64,
    pub money_infinite: PadBool<7>,
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
#[repr(C, packed(4))]
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
    pub max_capacity: f64,
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
