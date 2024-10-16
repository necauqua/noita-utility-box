use crate::memory::{ByteBool, PadBool, Ptr, RawPtr, StdMap, StdString, StdVec};
use derive_more::Debug;
use open_enum::open_enum;
use zerocopy::{FromBytes, IntoBytes};

use super::Vec2;

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct CellFactory {
    field_0x0: u32,
    pub material_ids: StdVec<StdString>,
    pub material_id_indices: StdMap<StdString, u32>,
    pub cell_data: StdVec<CellData>,
    pub number_of_materials: u32, // I mean this is the same as material_ids.len() but ok
    skip1: [u8; 0x68],
    pub materials_by_tag: StdMap<StdString, StdVec<Ptr<CellData>>>,
    pub reactions: StdVec<RawPtr>,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct CellData {
    pub name: StdString,
    pub ui_name: StdString,
    pub previous_id: i32,
    pub initial_id: i32,
    pub cell_type: CellType,
    pub platform_type: i32,
    pub wang_color: Color,
    pub gfx_glow: i32,
    pub gfx_glow_color: Color,
    pub graphics: CellGraphics,
    pub cell_holes_in_texture: ByteBool,
    pub stainable: ByteBool,
    pub burnable: ByteBool,
    pub on_fire: ByteBool,
    pub fire_hp: i32,
    pub autoignition_temperature: i32,
    pub hundred_minus_autoignition_temp: i32,
    pub temperature_of_fire: i32,
    pub generates_smoke: i32,
    pub generates_flames: i32,
    pub requires_oxygen: PadBool<3>,
    pub on_fire_convert_to_material: MaterialId,
    pub on_fire_flame_material: MaterialId,
    pub on_fire_smoke_material: MaterialId,
    pub explosion_config: Ptr<ConfigExplosion>,
    pub durability: i32,
    pub crackability: i32,
    pub electrical_conductivity: ByteBool,
    pub slippery: PadBool<2>,

    pub stickyness: f32,
    pub cold_freezes_to_material_name: StdString,
    pub warmth_melts_to_material: MaterialId,
    pub cold_freezes_to_material_id: u32,
    pub cold_freezes_chance_rev: i16,
    pub warmth_melts_chance_rev: i16,
    pub cold_freezes_to_dont_do_reverse_reaction: PadBool<3>,

    pub lifetime: i32,
    pub hp: i32,
    pub density: f32,
    pub liquid_sand: ByteBool,
    pub liquid_slime: ByteBool,
    pub liquid_static: ByteBool,
    pub liquid_stains_self: ByteBool,
    pub liquid_sticks_to_ceiling: i32,
    pub liquid_gravity: f32,
    pub liquid_viscosity: i32,
    pub liquid_stains: i32,
    pub liquid_stains_custom_color: Color,
    pub liquid_sprite_stain_shaken_drop_chance: f32,
    pub liquid_sprite_stain_ignited_drop_chance: f32,
    pub liquid_sprite_stains_check_offset: u8,
    #[debug(skip)]
    _pad: [u8; 3],

    pub liquid_sprite_stains_status_threshold: f32,
    pub liquid_damping: f32,
    pub liquid_flow_speed: f32,
    pub liquid_sand_never_box2d: PadBool<3>,

    pub gas_speed: u8,
    pub gas_upwards_speed: u8,
    pub gas_horizontal_speed: u8,
    pub gas_downwards_speed: u8,
    pub solid_friction: f32,
    pub solid_restitution: f32,
    pub solid_gravity_scale: f32,
    pub solid_static_type: i32,
    pub solid_on_collision_splash_power: f32,
    pub solid_on_collision_explode: ByteBool,
    pub solid_on_sleep_convert: ByteBool,
    pub solid_on_collision_convert: ByteBool,
    pub solid_on_break_explode: ByteBool,
    pub solid_go_through_sand: ByteBool,
    pub solid_collide_with_self: PadBool<2>,

    pub solid_on_collision_material: MaterialId,
    pub solid_break_to_type: MaterialId,
    pub convert_to_box2d_material: MaterialId,
    pub vegetation_full_lifetime_growth: i32,
    pub vegetation_sprite: StdString,
    pub vegetation_random_flip_x_scale: PadBool<3>,

    #[debug(skip)]
    _unknown: [u8; 0xc],

    pub wang_noise_percent: f32,
    pub wang_curvature: f32,
    pub wang_noise_type: i32,
    pub tags: StdVec<StdString>,
    pub danger_fire: ByteBool,
    pub danger_radioactive: ByteBool,
    pub danger_poison: ByteBool,
    pub danger_water: ByteBool,

    status_effects: RawPtr,
    #[debug(skip)]
    _unknown2: [u8; 0x14], // status_effects is prob a vector so some of this is it

    pub always_ignites_damagemodel: ByteBool,
    pub ignore_self_reaction_warning: PadBool<2>,

    pub audio_physics_material_event_idx: i32,
    pub audio_physics_material_wall_idx: i32,
    pub audio_physics_material_solid_idx: i32,
    pub audio_size_multiplier: f32,
    pub audio_is_soft: PadBool<3>,

    pub audio_materialaudio_type: i32,
    pub audio_materialbreakaudio_type: i32,
    pub show_in_creative_mode: ByteBool,
    pub is_just_particle_fx: ByteBool,
    pub transformed: PadBool<1>,

    pub particle_effect: Ptr<ParticleConfig>,
}
const _: () = assert!(std::mem::size_of::<CellData>() == 0x290);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct MaterialId {
    pub name: StdString,
    pub id: i32,
}

#[open_enum]
#[repr(u32)]
#[derive(FromBytes, IntoBytes, Debug)]
pub enum CellType {
    Liquid = 1,
    Gas,
    Solid,
    Fire,
}

#[derive(FromBytes, IntoBytes)]
#[repr(transparent)]
pub struct Color(pub u32);

impl Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let [r, g, b, a] = self.0.to_le_bytes();
        write!(f, "#{a:02x}{r:02x}{g:02x}{b:02x}")
    }
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct CellGraphics {
    pub texture_file: StdString,
    pub color: Color,
    pub fire_colors_index: u32,
    pub randomize_colors: ByteBool,
    pub normal_mapped: ByteBool,
    pub is_grass: ByteBool,
    pub is_grass_hashed: ByteBool,
    pub pixel_info: RawPtr,
    #[debug(skip)]
    _unknown: [u8; 0x18],
}
const _: () = assert!(std::mem::size_of::<CellGraphics>() == 0x40);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ConfigExplosion {
    pub vftable: RawPtr,
    pub never_cache: PadBool<3>,
    pub explosion_radius: f32,
    pub explosion_sprite: StdString,
    pub explosion_sprite_emissive: ByteBool,
    pub explosion_sprite_additive: ByteBool,
    pub explosion_sprite_random_rotation: PadBool<1>,
    pub explosion_sprite_lifetime: f32,
    pub damage: f32,
    pub damage_critical: ConfigDamageCritical,
    pub camera_shake: f32,
    pub particle_effect: PadBool<3>,
    pub load_this_entity: StdString,
    pub light_enabled: PadBool<3>,
    pub light_fade_time: f32,
    pub light_r: u32,
    pub light_g: u32,
    pub light_b: u32,
    pub light_radius_coeff: f32,
    pub hole_enabled: ByteBool,
    pub destroy_non_platform_solid_enabled: PadBool<2>,
    pub electricity_count: i32,
    pub min_radius_for_cracks: i32,
    pub crack_count: i32,
    pub knockback_force: f32,
    pub hole_destroy_liquid: ByteBool,
    pub hole_destroy_physics_dynamic: PadBool<2>,
    pub create_cell_material: StdString,
    pub create_cell_probability: i32,
    pub background_lightning_count: i32,
    pub spark_material: StdString,
    pub material_sparks_min_hp: i32,
    pub material_sparks_probability: i32,
    pub material_sparks_count: ValueRangeInt,
    pub material_sparks_enabled: ByteBool,
    pub material_sparks_real: ByteBool,
    pub material_sparks_scale_with_hp: ByteBool,
    pub sparks_enabled: ByteBool,
    pub sparks_count: ValueRangeInt,
    pub sparks_inner_radius_coeff: f32,
    pub stains_enabled: PadBool<3>,
    pub stains_radius: f32,
    pub ray_energy: i32,
    pub max_durability_to_destroy: i32,
    pub gore_particle_count: i32,
    pub shake_vegetation: ByteBool,
    pub damage_mortals: ByteBool,
    pub physics_throw_enabled: PadBool<1>,
    pub physics_explosion_power: ValueRange,
    pub physics_multiplier_ragdoll_force: f32,
    pub cell_explosion_power: f32,
    pub cell_explosion_radius_min: f32,
    pub cell_explosion_radius_max: f32,
    pub cell_explosion_velocity_min: f32,
    pub cell_explosion_damage_required: f32,
    pub cell_explosion_probability: f32,
    pub cell_power_ragdoll_coeff: f32,
    pub pixel_sprites_enabled: ByteBool,
    pub is_digger: ByteBool,
    pub audio_enabled: PadBool<1>,
    pub audio_event_name: StdString,
    pub audio_liquid_amount_normalized: f32,
    pub delay: ValueRangeInt,
    pub explosion_delay_id: i32,
    pub not_scaled_by_gamefx: PadBool<3>,
    pub who_is_responsible: u32,
    pub null_damage: PadBool<3>,
    pub dont_damage_this: u32,
    pub impl_send_message_to_this: u32,
    pub impl_position: Vec2,
    pub impl_delay_frame: i32,
}
const _: () = assert!(std::mem::size_of::<ConfigExplosion>() == 0x174);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ConfigDamageCritical {
    pub vftable: RawPtr,
    pub chance: i32,
    pub damage_multiplier: f32,
    pub m_succeeded: PadBool<3>,
}
const _: () = assert!(std::mem::size_of::<ConfigDamageCritical>() == 0x10);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ValueRange {
    pub min: f32,
    pub max: f32,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ValueRangeInt {
    pub min: i32,
    pub max: i32,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct Aabb {
    pub start: Vec2,
    pub end: Vec2,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ParticleConfig {
    pub vftable: RawPtr,
    pub m_material_id: i32,
    pub vel: Vec2,
    pub vel_random: Aabb,
    pub color: Color,
    pub lifetime: ValueRange,
    pub gravity: Vec2,
    pub cosmetic_force_create: ByteBool,
    pub render_back: ByteBool,
    pub render_on_grid: ByteBool,
    pub draw_as_long: ByteBool,
    pub airflow_force: f32,
    pub airflow_scale: f32,
    pub friction: f32,
    pub probability: f32,
    pub count: ValueRangeInt,
    pub particle_single_width: ByteBool,
    pub fade_based_on_lifetime: PadBool<2>,
}
const _: () = assert!(std::mem::size_of::<ParticleConfig>() == 0x54);
