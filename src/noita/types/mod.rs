use cell_factory::CellFactory;
use derive_more::Debug;
use std::{
    fmt::{self, Write as _},
    io,
    ops::Index,
};

use zerocopy::{FromBytes, IntoBytes};

use crate::memory::{
    Align4, ByteBool, MemoryStorage, PadBool, ProcessRef, Ptr, RawPtr, StdMap, StdString, StdVec,
};

pub mod cell_factory;
pub mod components;

#[derive(FromBytes, IntoBytes, Clone, Copy)]
#[repr(C)]
pub struct Bitset256([u8; 32]);

impl Index<u8> for Bitset256 {
    type Output = bool;

    // this actually never fails
    fn index(&self, index: u8) -> &Self::Output {
        if self.0[(index / 8) as usize] & (1 << (index % 8)) != 0 {
            &true
        } else {
            &false
        }
    }
}

impl Index<Option<u8>> for Bitset256 {
    type Output = bool;

    fn index(&self, index: Option<u8>) -> &Self::Output {
        index.map_or(&false, |i| &self[i])
    }
}

impl Debug for Bitset256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut res = String::with_capacity(256);
        for b in &self.0 {
            write!(&mut res, "{:08b}", b)?;
        }
        let cut = res.bytes().rposition(|b| b != b'0').unwrap_or(0);
        write!(f, "{}", &res[..=cut])
    }
}

#[derive(FromBytes, IntoBytes, Clone, Copy)]
#[repr(C)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Debug for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[derive(FromBytes, IntoBytes, Clone, Copy)]
#[repr(C)]
pub struct Vec2i {
    pub x: i32,
    pub y: i32,
}

impl Debug for Vec2i {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct EntityTransform {
    pub pos: Vec2,
    pub rot: Vec2,
    pub rot90: Vec2,
    pub scale: Vec2,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct Entity {
    pub id: u32,
    pub comp_idx: u32,
    pub filename_idx: u32,
    pub dead: PadBool<3>,
    field_0x10: u32,
    pub name: StdString,
    field_0x2c: u32,
    pub tags: Bitset256,
    pub transform: EntityTransform,
    pub children: Ptr<StdVec<Ptr<Entity>>>,
    pub parent: Ptr<Entity>,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct EntityManager {
    pub vftable: RawPtr,
    pub next_entity_id: u32,
    pub free_ids: StdVec<u32>,
    pub entities: StdVec<Ptr<Entity>>,
    pub entity_buckets: StdVec<StdVec<Ptr<Entity>>>,
    pub component_buffers: StdVec<Ptr<ComponentBuffer>>,
}

impl EntityManager {
    pub fn get_first_tagged_entity(
        &self,
        p: &ProcessRef,
        tag_index: u8,
    ) -> io::Result<Option<Ptr<Entity>>> {
        let Some(bucket) = self.entity_buckets.get(tag_index as u32) else {
            return Ok(None);
        };
        let Some(first) = bucket.read(p)?.get(0) else {
            return Ok(None);
        };
        Ok(Some(first.read(p)?))
    }
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct TagManager {
    pub tags: StdVec<StdString>,
    pub tag_indices: StdMap<StdString, u8>,
    pub max_tag_count: u32, // this is always 256 lul (and can't really be more cuz both bitset<256> and entity bucked idx being a byte)
    pub name: StdString,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct GameGlobal {
    pub frame_counter: u32,
    _skip: [u32; 5],
    pub cell_factory: Ptr<CellFactory>,
    _skip2: [u32; 97],
}
const _: () = assert!(std::mem::size_of::<GameGlobal>() == 0x1a0);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ComponentTypeManager {
    pub next_id: u32,
    pub component_indices: StdMap<StdString, u32>,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ComponentBuffer {
    pub vftable: RawPtr,
    pub default_index: u32,
    _skip1: [u8; 8],
    pub indices: StdVec<u32>,
    _skip2: [u8; 0x24],
    pub storage: StdVec<RawPtr>,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct GlobalStats {
    pub vftable: RawPtr,
    pub stats_version: u32,
    pub debug_tracker: u32,
    pub debug: PadBool<3>,
    pub debug_reset_counter: u32,
    pub fix_stats_flag: ByteBool,
    pub session_dead: PadBool<2>,
    pub key_value_stats: StdMap<StdString, u32>,
    pub session: GameStats,
    pub highest: GameStats,
    pub global: GameStats,
    pub prev_best: GameStats,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct GameStats {
    pub vftable: RawPtr,
    pub dead: PadBool<3>,
    pub death_count: u32,
    pub streaks: u32,
    pub world_seed: u32,
    pub killed_by: StdString,
    pub killed_by_extra: StdString,
    pub death_pos: Vec2,
    field_0x4c: u32, // 8-align padding?.
    pub playtime: f64,
    pub playtime_str: StdString,
    pub places_visited: u32,
    pub enemies_killed: u32,
    pub heart_containers: u32,
    field_0x7c: u32, // same?
    pub hp: i64,
    pub gold: i64,
    pub gold_all: i64,
    pub gold_infinite: PadBool<3>,
    pub items: u32,
    pub projectiles_shot: u32,
    pub kicks: u32,
    pub damage_taken: f64,
    pub healed: f64,
    pub teleports: u32,
    pub wands_edited: u32,
    pub biomes_visited_with_wands: u32,
    field_0xc4: u32, // same?
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct TranslationManager {
    pub vftable: RawPtr,
    pub unknown_strings: StdVec<StdString>,
    pub languages: StdVec<Language>,
    pub key_to_index: StdMap<StdString, u32>,
    pub extra_lang_files: StdVec<StdString>,
    pub current_lang_idx: u32,
    pub unknown: u32,
    pub unknown_float: f32,
    // those two are just empty in my game, so no clue what they are, no hints in ghidra (besides types) either
    pub unknown_primitive_vec: StdVec<u32>,
    pub unknown_map: StdMap<StdString, StdString>,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct Language {
    pub id: StdString,
    pub name: StdString,
    pub font_default: StdString,
    pub font_inventory_title: StdString,
    pub font_important_message_title: StdString,
    pub font_world_space_message: StdString,
    pub fonts_utf8: ByteBool,
    pub fonts_pixel_font: PadBool<2>,
    pub fonts_dpi: f32,
    pub ui_wand_info_offset1: f32,
    pub ui_wand_info_offset2: f32,
    pub ui_action_info_offset2: f32,
    pub ui_configurecontrols_offset2: f32,
    pub strings: StdVec<StdString>,
}
const _: () = assert!(std::mem::size_of::<Language>() == 0xb4);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct PlatformWin {
    pub vftable: RawPtr,
    pub application: RawPtr,
    pub app_config: RawPtr,
    pub internal_width: f32,
    pub internal_height: f32,
    pub input_disabled: PadBool<3>,
    pub graphics: RawPtr,
    pub fixed_time_step: PadBool<3>,
    pub frame_count: i32,
    pub frame_rate: i32,
    pub last_frame_execution_time: Align4<f64>,
    pub average_frame_execution_time: Align4<f64>,
    pub one_frame_should_last: Align4<f64>,
    pub time_elapsed_tracker: Align4<f64>,
    pub width: i32,
    pub height: i32,
    pub event_recorder: RawPtr,
    pub mouse: RawPtr,
    pub keyboard: RawPtr,
    pub touch: RawPtr,
    pub joysticks: StdVec<RawPtr>,
    pub sound_player: RawPtr,
    pub file_system: RawPtr,
    pub running: PadBool<3>,
    pub mouse_pos: Vec2,
    pub sleeping_mode: i32,
    pub print_framerate: PadBool<3>,
    pub working_dir: StdString,
    pub random_i: i32,
    pub random_seed: i32,
    pub joysticks_enabled: PadBool<3>,
}
const _: () = assert!(std::mem::size_of::<PlatformWin>() == 0xac);
