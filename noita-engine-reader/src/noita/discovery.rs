use std::{borrow::Cow, ffi::CStr};

use iced_x86::{Code, Instruction, OpKind, Register};
use memchr::memmem;

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
    match image.text()[image.find_push_str_pos(name)? - 8..] {
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

/// We look for the `AddFlagPersistent` Lua API function and then we look
/// for second-to-last `CALL rel32`, the last being some C++ exception
/// thing, and the second-to-last being a call to `AddFlagPersistentImpl`,
/// as I call it.
///
/// Then inside of that we look for `MOV ECX imm32` which is specifically
/// after `CALL rel32` which is after `MOV EDX, "progress_ending1"`.
/// The call being to a string equality check and our MOV being an
/// argument to a following call which is the global KEY_VALUE_STATS map
/// pointer.
fn find_stats_map_pointer(image: &ExeImage) -> Option<u32> {
    in_lua_api_fn(image, c"AddFlagPersistent")
        .filter(|instr| instr.code() == Code::Call_rel32_32)
        .forced_rev()
        .nth(1)?
        .jump_there(image)
        .skip_while({
            let end1_addr = image.find_string(c"progress_ending1")?;
            move |instr| {
                instr.code() != Code::Mov_r32_imm32
                    || instr.op0_register() != Register::EDX
                    || instr.immediate32() != end1_addr
            }
        })
        .skip_while(|instr| instr.code() != Code::Call_rel32_32)
        .find(|instr| instr.code() == Code::Mov_r32_imm32 && instr.op0_register() == Register::ECX)
        .map(|instr| instr.immediate32())
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
    let offset = image.find_push_str_pos(c"EntityTagManager")?;
    let addr = image.text_offset_to_addr(offset);
    image
        .decode_fn(addr)
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

/// In `GameTextGet` Lua API function:
///   - Look for the second CALL rel32 after JMP rm32 (second call after the
///     switch starts), which is a call to `Translate` function
///   - In that function extract the translation manager pointer from
///     `TRANSLATION_MANAGER.langs[TRANSLATION_MANAGER.current_lang_idx]`
///     pseudocode
fn find_translation_manager_pointer(image: &ExeImage) -> Option<u32> {
    in_lua_api_fn(image, c"GameTextGet")
        .skip_while(|instr| instr.code() != Code::Jmp_rm32)
        .skip_while(|instr| instr.code() != Code::Call_rel32_32)
        .skip(1)
        .find(|instr| instr.code() == Code::Call_rel32_32)?
        .jump_there(image)
        .find(|instr| instr.code() == Code::Add_r32_rm32 && instr.op0_register() == Register::EAX)
        .map(|instr| instr.memory_displacement32() - 0x10)
}

/// In `GameGetRealWorldTimeSinceStarted` Lua API function:
///  - Look for the last `MOV ECX, imm32` instruction, which is the
///    first arg to the vftable platform call (to get the time, duh).
fn find_platform_pointer(image: &ExeImage) -> Option<u32> {
    in_lua_api_fn(image, c"GameGetRealWorldTimeSinceStarted")
        .filter(|instr| {
            instr.code() == Code::Mov_r32_imm32 && instr.op0_register() == Register::ECX
        })
        .last()
        .map(|instr| instr.immediate32())
}

/// It's actually almost same as the PE timestamp I've been using, but
/// they might have some more human-readable stuff here.
pub fn find_noita_build(image: &ExeImage) -> Option<Cow<str>> {
    let pos = memmem::find(image.rdata(), b"Noita - Build ")?;

    // + 8 to skip the "Noita - " part
    let prefix = image.rdata()[pos + 8..].split(|b| *b == 0).next()?;
    Some(String::from_utf8_lossy(prefix))
}

pub fn run(image: &ExeImage) -> NoitaGlobals {
    let mut g = NoitaGlobals::default();

    let seed = find_seed_pointers(image);
    g.world_seed = seed.map(|(seed, _)| seed).map(|p| p.into());
    g.ng_count = seed.map(|(_, ng)| ng).map(|p| p.into());
    g.global_stats = find_stats_map_pointer(image).map(|p| (p - 0x18).into());
    g.game_global = find_game_global_pointer(image).map(|p| p.into());
    g.entity_manager = find_entity_manager_pointer(image).map(|p| p.into());
    g.entity_tag_manager = find_entity_tag_manager_pointer(image).map(|p| p.into());
    g.component_type_manager = find_component_type_manager_pointer(image).map(|p| p.into());
    g.translation_manager = find_translation_manager_pointer(image).map(|p| p.into());
    g.platform = find_platform_pointer(image).map(|p| p.into());

    g
}

#[allow(non_camel_case_types)]
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum KnownBuild {
    v2024_08_12 = 0x66ba59d6,
    v2025_01_25 = 0x6794ee3c,
}

impl KnownBuild {
    pub fn last() -> Self {
        Self::v2025_01_25
    }

    pub fn from_timestamp(timestamp: u32) -> Option<Self> {
        Some(Self::v2024_08_12).filter(|b| *b as u32 == timestamp)?;
        Some(Self::v2025_01_25).filter(|b| *b as u32 == timestamp)?;
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
                game_global: Some(Ptr::of(0x0122172c)),
                entity_manager: Some(Ptr::of(0x1202b78)),
                entity_tag_manager: Some(Ptr::of(0x1204fbc)),
                component_type_manager: Some(Ptr::of(0x01221c08)),
                translation_manager: Some(Ptr::of(0x01205c08)),
                platform: Some(Ptr::of(0x0121fba0)),
            },
            KnownBuild::v2025_01_25 => NoitaGlobals {
                world_seed: Some(Ptr::of(0x1205004)),
                ng_count: Some(Ptr::of(0x1205024)),
                global_stats: Some(Ptr::of(0x1208940)),
                game_global: Some(Ptr::of(0x122374c)),
                entity_manager: Some(Ptr::of(0x1204b98)),
                entity_tag_manager: Some(Ptr::of(0x1206fac)),
                component_type_manager: Some(Ptr::of(0x1223c88)),
                translation_manager: Some(Ptr::of(0x1207c28)),
                platform: Some(Ptr::of(0x1221bc0)),
            },
        }
    }
}
