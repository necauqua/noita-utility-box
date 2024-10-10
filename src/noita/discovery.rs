use std::{borrow::Cow, ffi::CStr};

use iced_x86::{Code, Register};
use memchr::memmem;

use crate::memory::exe_image::ExeImage;

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

/// We look for the `SetRandomSeed` Lua API function and then we look for
/// the `mov eax, [addr]` and `add eax, [addr]` instructions, which
/// correspond to WORLD_SEED + NEW_GAME_PLUS_COUNT being passed as a second
/// parameter of a (SetRandomSeedImpl) function call.
fn find_seed_pointers(image: &ExeImage) -> Option<(u32, u32)> {
    let mut state = None;
    for instr in image.decode_fn(find_lua_api_fn(image, c"SetRandomSeed")?) {
        state = match state {
            None if instr.code() == Code::Mov_EAX_moffs32 => Some(instr.memory_displacement32()),
            // allow the `add esp, 0x10` thing in between
            Some(addr) if instr.code() == Code::Add_rm32_imm8 => Some(addr),
            Some(addr)
                if instr.code() == Code::Add_r32_rm32 && instr.op0_register() == Register::EAX =>
            {
                return Some((addr, instr.memory_displacement32()));
            }
            _ => None,
        };
    }
    None
}

/// We look for the `GamePrint` Lua API function and then we look at the third
/// `CALL rel32` instruction from the end, which is a call to `GetGameGlobal`
/// (as I call it).
///
/// Then we look for the `MOV moffs32, EAX` instruction which is the assignment
/// to the pointer of the GameGlobal structure.
fn find_game_global_pointer(image: &ExeImage) -> Option<u32> {
    let third_from_last_call_rel = image
        .decode_fn(find_lua_api_fn(image, c"GamePrint")?)
        .filter(|instr| instr.code() == Code::Call_rel32_32)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .nth(2)?;

    image
        .decode_fn(third_from_last_call_rel.near_branch32())
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
    let mut before_last_call_rel = None;
    let mut last_call_rel = None;
    for instr in image.decode_fn(find_lua_api_fn(image, c"AddFlagPersistent")?) {
        if instr.code() == Code::Call_rel32_32 {
            before_last_call_rel = last_call_rel;
            last_call_rel = Some(instr.near_branch32());
        }
    }

    let end1_addr = image.find_string(c"progress_ending1")?;

    enum State {
        Init,
        FoundProgressEnding1,
        FoundStreqCall,
    }
    let mut state = State::Init;

    for instr in image.decode_fn(before_last_call_rel?) {
        match state {
            State::Init
                if instr.code() == Code::Mov_r32_imm32
                    && instr.op0_register() == Register::EDX
                    && instr.immediate32() == end1_addr =>
            {
                state = State::FoundProgressEnding1;
            }
            State::FoundProgressEnding1 if instr.code() == Code::Call_rel32_32 => {
                state = State::FoundStreqCall;
            }
            State::FoundStreqCall
                if instr.code() == Code::Mov_r32_imm32 && instr.op0_register() == Register::ECX =>
            {
                return Some(instr.immediate32());
            }
            _ => {}
        };
    }
    None
}

/// We look for the `EntityGetParent` Lua API function and there we look
/// for `MOV ECX, [addr]` which immediately follows a Lua call - that MOV
/// happens to be setting up an argument to a following relative call that
/// is the pointer to the entity manager global.
fn find_entity_manager_pointer(image: &ExeImage) -> Option<u32> {
    let mut state = false;

    for instr in image.decode_fn(find_lua_api_fn(image, c"EntityGetParent")?) {
        state = match state {
            false if instr.code() == Code::Call_rm32 => true,
            true if instr.code() == Code::Mov_r32_rm32 && instr.op0_register() == Register::ECX => {
                return Some(instr.memory_displacement32());
            }
            _ => false,
        };
    }
    None
}

/// Look for the `EntityHasTag` Lua API function and then look for the
/// second to last `CALL rel32` again, which is a call that accepts the
/// entity tag manager global in ECX
fn find_entity_tag_manager_pointer(image: &ExeImage) -> Option<u32> {
    let mut before_last_call_rel = None;
    let mut last_call_rel = None;

    let instrs = image
        .decode_fn(find_lua_api_fn(image, c"EntityHasTag")?)
        .enumerate()
        .map(|(i, instr)| {
            if instr.code() == Code::Call_rel32_32 {
                before_last_call_rel = last_call_rel;
                last_call_rel = Some(i);
            }
            instr
        })
        .collect::<Vec<_>>();

    instrs[..before_last_call_rel?]
        .iter()
        .rev()
        .find(|instr| instr.code() == Code::Mov_r32_rm32 && instr.op0_register() == Register::ECX)
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

    for instr in image.decode_fn(find_lua_api_fn(image, c"EntityGetComponent")?) {
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

    g
}

#[cfg(test)]
mod tests {
    use crate::memory::exe_image::PeHeader;

    use super::*;

    use std::time::Instant;

    use sysinfo::ProcessesToUpdate;
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::EnvFilter;

    #[test]
    fn test() -> anyhow::Result<()> {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::DEBUG.into())
                    .from_env()?,
            )
            .init();

        let mut system = sysinfo::System::new();
        system.refresh_processes(ProcessesToUpdate::All, true);

        let Some(noita_pid) = system
            .processes_by_exact_name("noita.exe".as_ref())
            .find(|p| p.thread_kind().is_none())
        else {
            eprintln!("Noita process not found");
            return Ok(());
        };

        let proc = noita_pid.pid().as_u32().try_into()?;
        let header = PeHeader::read(&proc)?;
        if header.timestamp() != 0x66ba59d6 {
            eprintln!("Timestamp mismatch: 0x{:x}", header.timestamp());
            return Ok(());
        }

        let instant = Instant::now();
        let image = header.read_image(&proc)?;
        println!("Image read in {:?}", instant.elapsed());

        let instant = Instant::now();
        let globals = run(&image);
        println!("Pointers found in {:?}", instant.elapsed());

        println!("{globals:#?}");

        // destructure so we know to update this when growing the list lol
        let NoitaGlobals {
            world_seed,
            ng_count,
            global_stats,
            game_global,
            entity_manager,
            entity_tag_manager,
            component_type_manager,
        } = NoitaGlobals::debug();

        assert_eq!(globals.world_seed, world_seed);
        assert_eq!(globals.ng_count, ng_count);
        assert_eq!(globals.global_stats, global_stats);
        assert_eq!(globals.game_global, game_global);
        assert_eq!(globals.entity_manager, entity_manager);
        assert_eq!(globals.entity_tag_manager, entity_tag_manager);
        assert_eq!(globals.component_type_manager, component_type_manager);

        Ok(())
    }
}
