use anyhow::Result;
use noita_engine_reader::memory::set_debug_process;

mod common;

#[test]
#[ignore] // manual
fn test() -> Result<()> {
    let mut noita = common::setup()?;
    set_debug_process(noita.proc().clone());

    // let ws = Ptr::<Ptr<Component<WorldStateComponent>>>::of(0x01202ff0);
    // println!("{:#?}", { ws.read(&proc)?.read(&proc)?.data });

    let player = noita.get_player()?.unwrap();
    println!("{player:#?}");

    let game_camera = noita.get_camera_pos();

    println!("camera: {game_camera:?}");

    Ok(())
}
