use cell_factory::CellFactory;
use derive_more::Debug;
use nub_macros::PtrReadable;
use std::{
    fmt::{self, Write as _},
    io,
    ops::Index,
};

use zerocopy::{FromBytes, IntoBytes};

use crate::memory::{
    ByteBool, MemoryStorage, PadBool, ProcessRef, Ptr, RawPtr, StdMap, StdString, StdVec, Vftable,
};

pub mod cell_factory;
pub mod components;
pub mod platform;

#[derive(FromBytes, IntoBytes, Clone, Copy)]
#[repr(C)]
pub struct Bitset<const N: usize>([u8; N]);

pub type Bitset256 = Bitset<32>;
pub type Bitset512 = Bitset<64>;

impl<const N: usize> Index<usize> for Bitset<N> {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        if self.0[(index / 8) as usize] & (1 << (index % 8)) != 0 {
            &true
        } else {
            &false
        }
    }
}

impl<const N: usize> Index<Option<usize>> for Bitset<N> {
    type Output = bool;

    fn index(&self, index: Option<usize>) -> &Self::Output {
        index.map_or(&false, |i| &self[i])
    }
}

impl<const N: usize> std::fmt::Debug for Bitset<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut res = String::with_capacity(N);
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

impl std::fmt::Debug for Vec2 {
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

impl std::fmt::Debug for Vec2i {
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
    pub tags: Bitset512,
    pub transform: EntityTransform,
    pub children: Ptr<StdVec<Ptr<Entity>>>,
    pub parent: Ptr<Entity>,
}

impl MemoryStorage for Ptr<Entity> {
    type Value = Entity;

    #[track_caller]
    fn read(&self, proc: &ProcessRef) -> io::Result<Self::Value> {
        // build 2025-01-25 updated the tag bitset to 512
        if proc.header().timestamp() >= 0x6794ee3c {
            return self.raw().read(proc);
        }

        #[derive(FromBytes, IntoBytes)]
        #[repr(C)]
        pub struct OldEntity {
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

        let old: OldEntity = self.raw().read(proc)?;
        let mut tags = Bitset([0; 64]);
        tags.0[..32].copy_from_slice(&old.tags.0);

        Ok(Entity {
            id: old.id,
            comp_idx: old.comp_idx,
            filename_idx: old.filename_idx,
            dead: old.dead,
            field_0x10: old.field_0x10,
            name: old.name,
            field_0x2c: old.field_0x2c,
            tags,
            transform: old.transform,
            children: old.children,
            parent: old.parent,
        })
    }
}

#[derive(Debug, PtrReadable)]
#[repr(C)]
pub struct EntityManager {
    pub vftable: Vftable,
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

#[derive(Debug, PtrReadable)]
#[repr(C)]
pub struct TagManager {
    pub tags: StdVec<StdString>,
    pub tag_indices: StdMap<StdString, u8>, // hmm, tag indices could be >256 now, we're truncating them..
    pub max_tag_count: u32,
    pub name: StdString,
}

#[derive(Debug, PtrReadable)]
#[repr(C)]
pub struct GameGlobal {
    pub frame_counter: u32,
    _skip: [u32; 5],
    pub cell_factory: Ptr<CellFactory>,
    _skip2: [u32; 97],
}
const _: () = assert!(std::mem::size_of::<GameGlobal>() == 0x1a0);

#[derive(Debug, PtrReadable)]
#[repr(C)]
pub struct ComponentTypeManager {
    pub next_id: u32,
    pub component_indices: StdMap<StdString, u32>,
}

#[derive(Debug, PtrReadable)]
#[repr(C)]
pub struct ComponentBuffer {
    pub vftable: Vftable,
    pub default_index: u32,
    _skip1: [u8; 8],
    pub indices: StdVec<u32>,
    _skip2: [u8; 0x24],
    pub storage: StdVec<RawPtr>,
}

#[derive(Debug, PtrReadable)]
#[repr(C)]
pub struct GlobalStats {
    pub vftable: Vftable,
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
    pub vftable: Vftable,
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

#[derive(Debug, PtrReadable)]
#[repr(C)]
pub struct TranslationManager {
    pub vftable: Vftable,
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

#[derive(Debug, PtrReadable)]
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
