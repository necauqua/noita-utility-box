use std::time::Instant;

use anyhow::{Result, bail};
use noita_engine_reader::{
    NoitaGlobals,
    discovery::{self, KnownBuild},
    memory::exe_image::ExeImage,
};

mod common;

#[test]
#[ignore] // manual
fn test() -> Result<()> {
    let noita = common::setup()?;
    let header = noita.proc().header();
    if header.timestamp() != KnownBuild::last().timestamp() {
        bail!("Timestamp mismatch: 0x{:x}", header.timestamp());
    }

    let instant = Instant::now();
    let image = ExeImage::read(&noita.proc())?;
    println!("Image read in {:?}", instant.elapsed());

    let instant = Instant::now();
    let globals = discovery::run(&image);
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
        translation_manager,
        platform,
    } = KnownBuild::last().map();

    assert_eq!(globals.world_seed, world_seed);
    assert_eq!(globals.ng_count, ng_count);
    assert_eq!(globals.global_stats, global_stats);
    assert_eq!(globals.game_global, game_global);
    assert_eq!(globals.entity_manager, entity_manager);
    assert_eq!(globals.entity_tag_manager, entity_tag_manager);
    assert_eq!(globals.component_type_manager, component_type_manager);
    assert_eq!(globals.translation_manager, translation_manager);
    assert_eq!(globals.platform, platform);

    Ok(())
}
