use std::io;

use crate::memory::{
    ByteBool, MemoryStorage, PadBool, Pod, ProcessRef, Ptr, RawPtr, StdMap, StdString, StdVec,
    Vftable,
};
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

    _unknown: u32,

    pub reaction_lookup: ReactionLookupTable,
    pub fast_reaction_lookup: ReactionLookupTable,
    pub req_reactions: StdVec<CellReactionBuf>,
    pub materials_by_tag: StdMap<StdString, StdVec<Ptr<CellData>>>,

    _unknown2: StdVec<Ptr<StdVec<RawPtr>>>, // we know this is vector< vector<something>* >

    pub fire_cell_data: Ptr<CellData>,

    _unknown3: [u32; 4],

    pub fire_material_id: u32,
}

impl CellFactory {
    /// This can be slow
    pub fn all_reactions(&self, proc: &ProcessRef) -> io::Result<Vec<CellReaction>> {
        let mut res = self.reaction_lookup.all_reactions(proc)?;
        res.extend(self.fast_reaction_lookup.all_reactions(proc)?);

        let req_reactions = self.req_reactions.read(proc)?;
        for buf in req_reactions {
            res.extend(buf.read(proc)?);
        }

        Ok(res)
    }

    pub fn lookup_reaction(&self, proc: &ProcessRef, input: u32) -> io::Result<Vec<CellReaction>> {
        let mut res = self.reaction_lookup.lookup(proc, input)?;
        res.extend(self.fast_reaction_lookup.lookup(proc, input)?);
        Ok(res)
    }
}

#[derive(FromBytes, IntoBytes, Debug, Clone)]
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
    pub max_reaction_probability: u32,
    pub max_fast_reaction_probability: u32,

    pub unknown_field: i32,

    pub wang_noise_percent: f32,
    pub wang_curvature: f32,
    pub wang_noise_type: i32,
    pub tags: StdVec<StdString>,
    pub danger_fire: ByteBool,
    pub danger_radioactive: ByteBool,
    pub danger_poison: ByteBool,
    pub danger_water: ByteBool,
    pub stain_effects: StdVec<StatusEffect>,
    pub ingestion_effects: StdVec<StatusEffect>,
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

#[derive(FromBytes, IntoBytes, Clone)]
#[repr(C)]
pub struct MaterialId {
    pub name: StdString,
    pub id: i32,
}

impl std::fmt::Debug for MaterialId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.name.is_empty() {
            match self.id {
                -1 => write!(f, "MaterialId::Air"),
                0 => write!(f, "MaterialId::None"),
                id => f.debug_tuple("MaterialId").field(&id).finish(),
            }
        } else {
            f.debug_tuple("MaterialId")
                .field(&self.name)
                .field(&self.id)
                .finish()
        }
    }
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct StatusEffect {
    pub id: i32,
    pub duration: f32,
}

#[open_enum]
#[repr(u32)]
#[derive(FromBytes, IntoBytes, Debug, Clone, Copy)]
pub enum CellType {
    Liquid = 1,
    Gas,
    Solid,
    Fire,
}

#[derive(FromBytes, IntoBytes, Clone, Copy)]
#[repr(transparent)]
pub struct Color(pub u32);

impl From<Color> for eframe::egui::Color32 {
    fn from(value: Color) -> Self {
        let [r, g, b, a] = value.0.to_le_bytes();
        Self::from_rgba_premultiplied(r, g, b, a)
    }
}

impl std::fmt::Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let [r, g, b, a] = self.0.to_le_bytes();
        write!(f, "#{a:02x}{r:02x}{g:02x}{b:02x}")
    }
}

#[derive(FromBytes, IntoBytes, Debug, Clone)]
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
    pub vftable: Vftable,
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
    pub vftable: Vftable,
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
    pub vftable: Vftable,
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

#[open_enum]
#[repr(i32)]
#[derive(FromBytes, IntoBytes, Debug)]
pub enum ReactionDir {
    None = 1 - 2, // plain -1 does not work cuz open_enum is bugged lol
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct CellReaction {
    pub fast_reaction: PadBool<3>,
    pub probability_times_100: u32,
    pub input_cell1: i32,
    pub input_cell2: i32,
    pub output_cell1: i32,
    pub output_cell2: i32,
    pub has_input_cell3: PadBool<3>,
    pub input_cell3: i32,
    pub output_cell3: i32,
    pub cosmetic_particle: i32,
    pub req_lifetime: i32,
    pub blob_radius1: u8,
    pub blob_radius2: u8,
    pub blob_restrict_to_input_material1: ByteBool,
    pub blob_restrict_to_input_material2: ByteBool,
    pub destroy_horizontally_lonely_pixels: ByteBool,
    pub convert_all: PadBool<2>,
    pub entity_file_idx: u32,
    pub direction: ReactionDir,
    pub explosion_config: Ptr<ConfigExplosion>,
    pub audio_fx_volume_1: f32,
}
const _: () = assert!(std::mem::size_of::<CellReaction>() == 0x44);

impl CellReaction {
    pub fn pretty_print(&self, materials: &[String]) -> String {
        use std::fmt::Write;

        fn name(materials: &[String], id: i32) -> &str {
            materials.get(id as usize).map_or("unknown", |s| s.as_str())
        }

        let mut res = String::new();
        let _ = write!(
            &mut res,
            "{} + {}",
            name(materials, self.input_cell1),
            name(materials, self.input_cell2),
        );
        if self.has_input_cell3.get().as_bool() {
            let _ = write!(&mut res, " + {}", name(materials, self.input_cell3));
        }
        let _ = write!(
            &mut res,
            " => {} + {}",
            name(materials, self.output_cell1),
            name(materials, self.output_cell2),
        );
        if self.output_cell3 != -1 {
            let _ = write!(&mut res, " + {}", name(materials, self.output_cell3));
        }
        if self.cosmetic_particle != -1 {
            let _ = write!(&mut res, " ^{}", name(materials, self.cosmetic_particle));
        }
        let _ = write!(
            &mut res,
            " : {}%",
            self.probability_times_100 as f32 / 100.0
        );
        res
    }
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct CellReactionBuf {
    base: Ptr<CellReaction>,
    _unknown: u32, // only ever saw this equal to len
    len: u32,
}

impl CellReactionBuf {
    pub const fn len(&self) -> u32 {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.base.is_null() || self.len == 0
    }

    pub fn get(&self, index: u32) -> Option<Ptr<CellReaction>> {
        (index < self.len).then(|| self.base.offset(index as i32))
    }
}

impl MemoryStorage for CellReactionBuf {
    type Value = Vec<CellReaction>;

    fn read(&self, proc: &ProcessRef) -> io::Result<Self::Value> {
        if self.is_empty() {
            return Ok(Vec::new());
        }
        self.base.raw().read_multiple(proc, self.len)
    }
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ReactionLookupTable {
    pub width: u32,
    pub height: u32,
    pub len: u32,
    // #[debug(skip)]
    _unknown: [u32; 5],
    storage: Ptr<CellReactionBuf>,
    _unknown2: u32,
    _unknown3: u32,
}

impl ReactionLookupTable {
    pub fn lookup(&self, proc: &ProcessRef, material_id: u32) -> io::Result<Vec<CellReaction>> {
        let mut result = Vec::new();
        for i in 0..self.height {
            let reactions = self
                .storage
                .offset((self.width * i + material_id) as _)
                .read(proc)?
                .read(proc)?;
            result.extend(reactions);
        }
        Ok(result)
    }

    pub fn all_reactions(&self, proc: &ProcessRef) -> io::Result<Vec<CellReaction>> {
        let mut result = Vec::new();
        for b in proc.read_multiple::<CellReactionBuf>(self.storage.addr(), self.len)? {
            result.extend(b.read(proc)?);
        }
        Ok(result)
    }
}

// So ReactionLookupTable is supposed to be CArray2D<CSafeArray<CellReaction>>
// but something doesn't add up yet

#[derive(FromBytes, IntoBytes, Clone, Copy, std::fmt::Debug)]
#[repr(C, packed)]
pub struct CSafeArray<T> {
    pub data: Ptr<T>,
    pub len: u32,
}

impl<T> CSafeArray<T> {
    pub const fn is_empty(&self) -> bool {
        self.len == 0 || { self.data }.is_null()
    }

    pub const fn truncate(&self, new_len: u32) -> Self {
        Self {
            data: self.data,
            len: new_len,
        }
    }

    pub const fn slice(&self, offset: u32, len: u32) -> Self {
        Self {
            data: self.data.offset(offset as i32),
            len,
        }
    }
}

impl<T: Pod> MemoryStorage for CSafeArray<T> {
    type Value = Vec<T>;

    fn read(&self, proc: &ProcessRef) -> io::Result<Self::Value> {
        if self.is_empty() {
            return Ok(Vec::new());
        }
        self.data.raw().read_multiple(proc, self.len)
    }
}

#[derive(FromBytes, IntoBytes)]
#[repr(C, packed)]
pub struct CArray2D<T> {
    pub width: u32,
    pub height: u32,
    pub size: u32,

    pub helper: CArray2DHelper<T>,
    not_sure: [u32; 2],
    null_pointer_thing: RawPtr, // ?

    storage: CSafeArray<T>,
}

#[derive(FromBytes, IntoBytes)]
#[repr(C, packed)]
pub struct CArray2DHelper<T> {
    pub x: u32,
    pub array: Ptr<CArray2D<T>>,
}
