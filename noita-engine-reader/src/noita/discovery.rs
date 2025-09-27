use std::{borrow::Cow, ffi::CStr};

use iced_x86::{Code, Instruction, OpKind, Register};

use crate::memory::{Ptr, exe_image::ExeImage};

use super::NoitaGlobals;

/// Assuming Lua API functions are set up like this..
/// ```c
///   lua_pushcclosure(L,function_pointer,0);
///   lua_setfield(L,LUA_GLOBALSINDEX,"UniqueString");
/// ```
/// ..we look for the `PUSH imm32` of the unique string given as `name`, and
/// then we look if there is a `PUSH imm32` at 8 bytes before that
/// (`CALL EDI => lua_pushcclosure` and `PUSH EBX` being 3 bytes, and
/// 5 bytes for the `PUSH imm32` image), and return it's argument.
///
/// Note that this completely breaks (already) with noita_dev.exe lol
fn find_lua_api_fn(image: &ExeImage, name: &CStr) -> Option<u32> {
    match image[image.find_push_str(name)? - image.base() - 8..] {
        [0x68, a, b, c, d, ..] => {
            let addr = u32::from_le_bytes([a, b, c, d]);
            tracing::debug!("Found Lua API function {name:?} at 0x{addr:x}");
            Some(addr)
        }
        _ => {
            tracing::warn!("Did not find Lua API function {name:?}");
            None
        }
    }
}

/// Adapt the above function to return a stream of instructions
fn in_lua_api_fn<'a>(image: &'a ExeImage, name: &CStr) -> impl Iterator<Item = Instruction> + 'a {
    find_lua_api_fn(image, name)
        .map(|addr| image.decode_fn(addr))
        .into_iter()
        .flatten()
}

trait JumpThere {
    fn jump_there(self, image: &ExeImage) -> impl Iterator<Item = Instruction>;
}

impl JumpThere for Instruction {
    fn jump_there(self, image: &ExeImage) -> impl Iterator<Item = Instruction> {
        image.decode_fn(self.near_branch32())
    }
}

trait ForcedRev: Iterator {
    fn forced_rev(self) -> impl Iterator<Item = Self::Item>;
}

impl<I: Iterator> ForcedRev for I {
    fn forced_rev(self) -> impl Iterator<Item = Self::Item> {
        self.collect::<Vec<_>>().into_iter().rev()
    }
}

/// We look for the `SetRandomSeed` Lua API function and then we look for
/// the `mov eax, [addr]` and `add eax, [addr]` instructions, which
/// correspond to WORLD_SEED + NEW_GAME_PLUS_COUNT being passed as a second
/// parameter of a (SetRandomSeedImpl) function call.
fn find_seed_pointers(image: &ExeImage) -> Option<(u32, u32)> {
    let mut ng_plus = None;
    in_lua_api_fn(image, c"SetRandomSeed")
        .forced_rev()
        .skip_while(|instr| {
            if instr.code() != Code::Add_r32_rm32 || instr.op0_register() != Register::EAX {
                return true;
            }
            ng_plus = Some(instr.memory_displacement32());
            false
        })
        // allow the `add esp, 0x10` thing in between
        .skip_while(|instr| instr.code() == Code::Add_rm32_imm8)
        .find(|instr| instr.code() == Code::Mov_EAX_moffs32)
        .and_then(|instr| Some((instr.memory_displacement32(), ng_plus?)))
}

/// We look for the `GamePrint` Lua API function and then we look at the third
/// `CALL rel32` instruction from the end, which is a call to `GetGameGlobal`
/// (as I call it).
///
/// Then we look for the `MOV moffs32, EAX` instruction which is the assignment
/// to the pointer of the GameGlobal structure.
fn find_game_global_pointer(image: &ExeImage) -> Option<u32> {
    in_lua_api_fn(image, c"GamePrint")
        .filter(|instr| instr.code() == Code::Call_rel32_32)
        .forced_rev()
        .nth(2)?
        .jump_there(image)
        .find(|instr| {
            instr.code() == Code::Mov_moffs32_EAX && instr.segment_prefix() == Register::None
        })
        .map(|instr| instr.memory_displacement32())
}

/// We look for the `EntityGetParent` Lua API function and there we look
/// for `MOV ECX, [addr]` is the 0th argument to EntityManager::get_entity, the
/// entity manager global.
fn find_entity_manager_pointer(image: &ExeImage) -> Option<u32> {
    in_lua_api_fn(image, c"EntityGetParent")
        .find(|instr| {
            instr.code() == Code::Mov_r32_rm32
                && instr.op0_register() == Register::ECX
                && instr.op1_kind() == OpKind::Memory
        })
        .map(|instr| instr.memory_displacement32())
}

/// Look for the `EntityTagManager` string only use, and then look for the
/// following assignment to a global from EAX
fn find_entity_tag_manager_pointer(image: &ExeImage) -> Option<u32> {
    image
        .decode_fn(image.find_push_str(c"EntityTagManager")? as u32)
        .find(|instr| instr.code() == Code::Mov_moffs32_EAX)
        .map(|instr| instr.memory_displacement32())
}

/// Look for the `EntityGetComponent` Lua API function and then look for
/// a `CALL rel32` instruction that immediately follows a `PUSH EAX`,
/// it's a call to `GetComponentTypeManager` (as I call it).
///
/// Then we look for the `MOV EAX, imm32` instruction which the return
/// of the component type manager global pointer.
fn find_component_type_manager_pointer(image: &ExeImage) -> Option<u32> {
    let mut state = false;
    let mut found = None;

    // havent found a low-hanging streaming version of "find X that immediately follows Y"
    for instr in in_lua_api_fn(image, c"EntityGetComponent") {
        state = match state {
            false if instr.code() == Code::Push_r32 && instr.op0_register() == Register::EAX => {
                true
            }
            true if instr.code() == Code::Call_rel32_32 => {
                found = Some(instr.near_branch32());
                break;
            }
            _ => false,
        };
    }

    image
        .decode_fn(found?)
        .find(|instr| instr.code() == Code::Mov_r32_imm32)
        .map(|instr| instr.immediate32())
}

fn find_persistent_flag_manager_pointer(image: &ExeImage) -> Option<u32> {
    in_lua_api_fn(image, c"AddFlagPersistent")
        .filter(|instr| {
            instr.code() == Code::Mov_r32_rm32
                && instr.op0_register() == Register::ECX
                && instr.memory_base() == Register::None
        })
        .last()
        .map(|instr| instr.memory_displacement32())
}

/// It's actually almost same as the PE timestamp I've been using, but
/// they might have some more human-readable stuff here.
pub fn find_noita_build(image: &ExeImage) -> Option<Cow<'_, str>> {
    let addr = image.rdata().scan(b"Noita - Build ")?;
    let pos = addr - image.base();

    // + 8 to skip the "Noita - " part
    let prefix = image[pos + 8..].split(|b| *b == 0).next()?;
    Some(String::from_utf8_lossy(prefix))
}

pub fn run(image: &ExeImage) -> NoitaGlobals {
    let seed = find_seed_pointers(image);

    NoitaGlobals {
        world_seed: seed.map(|(seed, _)| seed).map(|p| p.into()),
        ng_count: seed.map(|(_, ng)| ng).map(|p| p.into()),
        global_stats: image
            .find_static_global(c".?AVGlobalStats@@")
            .map(|p| p.into()),
        config_player_stats: image
            .find_static_global(c".?AVConfigPlayerStats@impl@@")
            .map(|p| p.into()),
        game_global: find_game_global_pointer(image).map(|p| p.into()),
        entity_manager: find_entity_manager_pointer(image).map(|p| p.into()),
        entity_tag_manager: find_entity_tag_manager_pointer(image).map(|p| p.into()),
        component_type_manager: find_component_type_manager_pointer(image).map(|p| p.into()),
        translation_manager: image
            .find_static_global(c".?AUTextImpl@@")
            .map(|p| p.into()),
        platform: image
            .find_static_global(c".?AVPlatformWin@poro@@")
            .map(|p| p.into()),
        persistent_flag_manager: find_persistent_flag_manager_pointer(image).map(|p| p.into()),
        mod_context: image
            .find_static_global(c".?AUModContext@@")
            .map(|p| p.into()),
    }
}

#[allow(non_camel_case_types)]
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum KnownBuild {
    v2024_08_12 = 0x66ba59d6,
    v2025_01_25_beta = 0x6794c092,
    v2025_01_25 = 0x6794ee3c,
}

impl KnownBuild {
    pub fn last() -> Self {
        Self::v2025_01_25
    }

    // todo maybe macro this somehow?.
    pub fn from_timestamp(timestamp: u32) -> Option<Self> {
        if Self::v2024_08_12 as u32 == timestamp {
            return Some(Self::v2024_08_12);
        }
        if Self::v2025_01_25_beta as u32 == timestamp {
            return Some(Self::v2025_01_25_beta);
        }
        if Self::v2025_01_25 as u32 == timestamp {
            return Some(Self::v2025_01_25);
        }
        None
    }

    pub fn timestamp(self) -> u32 {
        self as u32
    }

    pub fn map(self) -> NoitaGlobals {
        match self {
            KnownBuild::v2024_08_12 => NoitaGlobals {
                world_seed: Some(Ptr::of(0x1202fe4)),
                ng_count: Some(Ptr::of(0x1203004)),
                global_stats: Some(Ptr::of(0x1206920)),
                config_player_stats: Some(Ptr::of(0x1206740)),
                game_global: Some(Ptr::of(0x122172c)),
                entity_manager: Some(Ptr::of(0x1202b78)),
                entity_tag_manager: Some(Ptr::of(0x1204fbc)),
                component_type_manager: Some(Ptr::of(0x1221c08)),
                translation_manager: Some(Ptr::of(0x1205c08)),
                platform: Some(Ptr::of(0x121fba0)),
                persistent_flag_manager: Some(Ptr::of(0x12053cc)),
                mod_context: Some(Ptr::of(0x1205e60)),
            },
            KnownBuild::v2025_01_25_beta | KnownBuild::v2025_01_25 => NoitaGlobals {
                world_seed: Some(Ptr::of(0x1205004)),
                ng_count: Some(Ptr::of(0x1205024)),
                global_stats: Some(Ptr::of(0x1208940)),
                config_player_stats: Some(Ptr::of(0x1208760)),
                game_global: Some(Ptr::of(0x122374c)),
                entity_manager: Some(Ptr::of(0x1204b98)),
                entity_tag_manager: Some(Ptr::of(0x1206fac)),
                component_type_manager: Some(Ptr::of(0x1223c88)),
                translation_manager: Some(Ptr::of(0x1207c28)),
                platform: Some(Ptr::of(0x1221bc0)),
                persistent_flag_manager: Some(Ptr::of(0x12073f4)),
                mod_context: Some(Ptr::of(0x1207e80)),
            },
        }
    }
}
