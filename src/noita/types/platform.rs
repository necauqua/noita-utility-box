use std::io;

use open_enum::open_enum;
use zerocopy::{FromBytes, IntoBytes};

use crate::memory::{
    Align4, ByteBool, MemoryStorage, ProcessRef, Ptr, Raw, RawPtr, StdMap, StdString, StdVec,
    StdWstring, Vftable, WithPad,
};

use super::{cell_factory::CSafeArray, Vec2};

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct PlatformWin {
    pub vftable: Vftable,
    pub application: RawPtr,
    pub app_config: Ptr<WizardAppConfig>,
    pub internal_width: f32,
    pub internal_height: f32,
    pub input_disabled: WithPad<ByteBool, 3>,
    pub graphics: RawPtr,
    pub fixed_time_step: WithPad<ByteBool, 3>,
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
    pub file_system: Ptr<FileSystem>,
    pub running: WithPad<ByteBool, 3>,
    pub mouse_pos: Vec2,
    pub sleeping_mode: i32,
    pub print_framerate: WithPad<ByteBool, 3>,
    pub working_dir: StdString,
    pub random_i: i32,
    pub random_seed: i32,
    pub joysticks_enabled: WithPad<ByteBool, 3>,
}
const _: () = assert!(std::mem::size_of::<PlatformWin>() == 0xac);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ControlsConfigKey {
    pub primary: i32,
    pub secondary: i32,
    pub primary_name: StdString,
    pub secondary_name: StdString,
}
const _: () = assert!(std::mem::size_of::<ControlsConfigKey>() == 0x38);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct ControlsConfig {
    pub key_up: ControlsConfigKey,
    pub key_down: ControlsConfigKey,
    pub key_left: ControlsConfigKey,
    pub key_right: ControlsConfigKey,
    pub key_use_wand: ControlsConfigKey,
    pub key_spray_flask: ControlsConfigKey,
    pub key_throw: ControlsConfigKey,
    pub key_kick: ControlsConfigKey,
    pub key_inventory: ControlsConfigKey,
    pub key_interact: ControlsConfigKey,
    pub key_drop_item: ControlsConfigKey,
    pub key_drink_potion: ControlsConfigKey,
    pub key_item_next: ControlsConfigKey,
    pub key_item_prev: ControlsConfigKey,
    pub key_item_slot1: ControlsConfigKey,
    pub key_item_slot2: ControlsConfigKey,
    pub key_item_slot3: ControlsConfigKey,
    pub key_item_slot4: ControlsConfigKey,
    pub key_item_slot5: ControlsConfigKey,
    pub key_item_slot6: ControlsConfigKey,
    pub key_item_slot7: ControlsConfigKey,
    pub key_item_slot8: ControlsConfigKey,
    pub key_item_slot9: ControlsConfigKey,
    pub key_item_slot10: ControlsConfigKey,
    pub key_takescreenshot: ControlsConfigKey,
    pub key_replayedit_open: ControlsConfigKey,
    pub aim_stick: ControlsConfigKey,
    pub key_ui_confirm: ControlsConfigKey,
    pub key_ui_drag: ControlsConfigKey,
    pub key_ui_quick_drag: ControlsConfigKey,
    pub gamepad_analog_sticks_threshold: f32,
    pub gamepad_analog_buttons_threshold: f32,
}
const _: () = assert!(std::mem::size_of::<ControlsConfig>() == 0x698);

#[open_enum]
#[repr(u32)]
#[derive(FromBytes, IntoBytes, Debug, Clone, Copy)]
pub enum VsyncMode {
    Off,
    On,
    Adaptive,
}

#[open_enum]
#[repr(u32)]
#[derive(FromBytes, IntoBytes, Debug, Clone, Copy)]
pub enum FullscreenMode {
    Windowed,
    Stretched,
    Full,
}

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct GraphicsSettings {
    pub window_w: u32,
    pub window_h: u32,
    pub fullscreen: FullscreenMode,
    pub caption: StdString,
    pub icon_bmp: StdString,
    pub textures_resize_to_power_of_two: ByteBool,
    pub textures_fix_alpha_channel: WithPad<ByteBool, 2>,
    pub vsync: VsyncMode,
    pub current_display: u32,
    pub external_graphics_context: RawPtr,
}
const _: () = assert!(std::mem::size_of::<GraphicsSettings>() == 0x4c);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct AppConfig {
    pub vftable: Vftable,
    pub internal_size_w: u32,
    pub internal_size_h: u32,
    pub framerate: u32,
    pub iphone_is_landscape: ByteBool,
    pub sounds: ByteBool,
    pub event_recorder_flush_every_frame: ByteBool,
    pub record_events: ByteBool,
    pub do_a_playback: WithPad<ByteBool, 3>,
    pub playback_file: StdString,
    pub report_fps: ByteBool,
    pub joysticks_enabled: WithPad<ByteBool, 2>,
    pub joystick_rumble_intensity: f32,
    pub graphics_settings: GraphicsSettings,
    pub set_random_seed_cb: RawPtr, // a function pointer (aka useless)
}
const _: () = assert!(std::mem::size_of::<AppConfig>() == 0x88);

#[derive(FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct WizardAppConfig {
    pub p: AppConfig,
    pub has_been_started_before: ByteBool,
    pub audio_fmod: WithPad<ByteBool, 2>,
    pub audio_music_volume: f32,
    pub audio_effects_volume: f32,
    pub rendering_low_quality: ByteBool,
    pub rendering_low_resolution: ByteBool,
    pub rendering_pixel_art_antialiasing: WithPad<ByteBool, 1>,
    pub rendering_brightness_delta: f32,
    pub rendering_contrast_delta: f32,
    pub rendering_gamma_delta: f32,
    pub rendering_teleport_flash_brightness: f32,
    pub rendering_cosmetic_particle_count_coeff: f32,
    pub backbuffer_width: i32,
    pub backbuffer_height: i32,
    pub application_rendered_cursor: WithPad<ByteBool, 3>,
    pub screenshake_intensity: f32,
    pub ui_inventory_icons_always_clickable: ByteBool,
    pub ui_allow_shooting_while_inventory_open: ByteBool,
    pub ui_report_damage: ByteBool,
    pub ui_show_world_hover_info_next_to_mouse: ByteBool,
    pub replay_recorder_enabled: WithPad<ByteBool, 3>,
    pub replay_recorder_max_budget_mb: u32,
    pub replay_recorder_max_resolution_x: u32,
    pub replay_recorder_max_resolution_y: u32,
    pub language: StdString,
    pub check_for_updates: WithPad<ByteBool, 3>,
    pub last_started_game_version_hash: StdString,
    pub config_format_version: u32,
    pub is_default_config: WithPad<ByteBool, 3>,
    pub keyboard_controls: ControlsConfig,
    pub gamepad_controls: ControlsConfig,
    pub gamepad_mode: i32,
    pub rendering_filmgrain: ByteBool,
    pub online_features: WithPad<ByteBool, 2>,
    pub steam_cloud_size_warning_limit_mb: f32,
    pub _unknown_bool: ByteBool,
    pub mouse_capture_inside_window: ByteBool,
    pub ui_snappy_hover_boxes: ByteBool,
    pub application_pause_when_unfocused: ByteBool,
    pub gamepad_analog_flying: WithPad<ByteBool, 3>,
    pub mods_active: StdString,
    pub mods_active_privileged: StdString,
    pub mods_sandbox_enabled: ByteBool,
    pub mods_sandbox_warning_done: ByteBool,
    pub mods_disclaimer_accepted: ByteBool,
    pub streaming_integration_autoconnect: ByteBool,
    pub streaming_integration_channel_name: StdString,
    pub streaming_integration_events_per_vote: u32,
    pub _unknown_streaming_number: f32,
    pub streaming_integration_time_seconds_voting: f32,
    pub streaming_integration_time_seconds_between_votings: f32,
    pub streaming_integration_play_new_vote_sound: ByteBool,
    pub streaming_integration_viewernames_ghosts: ByteBool,
    pub streaming_integration_hide_votes_during_voting: ByteBool,
    pub streaming_integration_ui_pos_left: ByteBool,
    pub single_threaded_loading: WithPad<ByteBool, 3>,
    pub _unknown_string: StdString,
    pub debug_dont_load_other_config: WithPad<ByteBool, 3>,
}
const _: () = assert!(std::mem::size_of::<WizardAppConfig>() == 0xed0);

#[derive(FromBytes, IntoBytes, derive_more::Debug, Clone)]
#[repr(C)]
pub struct FileSystem {
    pub devices: StdVec<RawPtr>,
    pub path_proxies: StdVec<PathProxy>,
    pub mutex: RawPtr,
    pub default_device: Ptr<DiskFileDevice>,
    pub default_device_2: Ptr<DiskFileDevice>,
}
const _: () = assert!(std::mem::size_of::<FileSystem>() == 0x24);

#[open_enum]
#[repr(u32)]
#[derive(FromBytes, IntoBytes, Debug, Clone, Copy)]
pub enum PathLocation {
    UserDirectory,    // appdata
    WorkingDirectory, // where the exe is
}

#[derive(FromBytes, IntoBytes, Debug, Clone)]
#[repr(C)]
pub struct PathProxy {
    pub name: StdString,
    pub location: PathLocation,
    pub path: StdString,
}

#[derive(FromBytes, IntoBytes, Debug, Clone)]
#[repr(C)]
pub struct ModDiskFileDeviceCaching {
    pub vftable: Vftable,
    pub entries: StdMap<StdString, Raw<ModFileEntry>>,
}

#[derive(FromBytes, IntoBytes, Debug, Clone, Copy)]
#[repr(C)]
pub struct ModFileEntry {
    pub filename: StdString,
    pub flag: WithPad<u8, 3>,
    pub mod_device: Ptr<ModDiskFileDevice>,
    pub cache: CSafeArray<u8>,
    pub unknown: i32,
    pub override_with: StdString,
}
const _: () = assert!(std::mem::size_of::<ModFileEntry>() == 0x44);

impl IFileDevice for ModDiskFileDeviceCaching {
    /// Loosely follows the ModDiskFileDeviceCaching::OpenRead
    fn get_file(
        &self,
        proc: &ProcessRef,
        fs: &FileSystem,
        path: &str,
    ) -> io::Result<Option<Vec<u8>>> {
        let Some(entry) = self.entries.get(proc, &path.to_lowercase())? else {
            return Ok(None);
        };

        // cache hit
        if !{ entry.cache.data }.is_null() {
            return entry.cache.read(proc).map(Some);
        }

        // override recurses
        if !entry.override_with.is_empty() {
            return self.get_file(proc, fs, &entry.override_with.read(proc)?);
        }

        if entry.mod_device.is_null() {
            return Ok(None);
        }

        let mod_device = entry.mod_device.read(proc)?;

        if entry.flag.get() != 0 {
            // the fabled 13th method of ModDiskFileDevice
            // aka fall through to its disk device
            return mod_device
                .disk_device
                .get_file(proc, fs, &entry.filename.read(proc)?);
        }

        mod_device.get_file(proc, fs, &entry.filename.read(proc)?)
    }
}

#[derive(FromBytes, IntoBytes, Debug, Clone)]
#[repr(C)]
pub struct WizardPakFileDevice {
    pub vftable: Vftable,
    pub _skip: u32, // unknown
    pub pak: WizardPak,
}

#[derive(FromBytes, IntoBytes, Debug, Clone)]
#[repr(C)]
pub struct WizardPak {
    pub data: CSafeArray<u8>,
    pub files: StdMap<StdString, Raw<WizardPakSlice>>,
    pub file_names: StdVec<StdString>,
}

#[derive(FromBytes, IntoBytes, Clone, Copy, Debug)]
#[repr(C)]
pub struct WizardPakSlice {
    pub offset: u32,
    pub len: u32,
}

impl IFileDevice for WizardPakFileDevice {
    fn get_file(
        &self,
        proc: &ProcessRef,
        _fs: &FileSystem,
        path: &str,
    ) -> io::Result<Option<Vec<u8>>> {
        let Some(entry) = self.pak.files.get(proc, path)? else {
            return Ok(None);
        };
        self.pak
            .data
            .slice(entry.offset, entry.len)
            .read(proc)
            .map(Some)
    }
}

#[derive(FromBytes, IntoBytes, Debug, Clone)]
#[repr(C)]
pub struct ModDiskFileDevice {
    pub vftable: Vftable,
    pub disk_device: DiskFileDevice,
    pub mod_path_prefix: StdString,
    pub mod_path_prefix_lowercase: StdString,
}

impl IFileDevice for ModDiskFileDevice {
    fn get_file(
        &self,
        proc: &ProcessRef,
        fs: &FileSystem,
        path: &str,
    ) -> io::Result<Option<Vec<u8>>> {
        let name = path.to_lowercase();
        let Some(name) = name.strip_prefix(&self.mod_path_prefix_lowercase.read(proc)?) else {
            return Ok(None);
        };
        self.disk_device.get_file(proc, fs, name)
    }
}

#[derive(FromBytes, IntoBytes, Debug, Clone)]
#[repr(C)]
pub struct DiskFileDevice {
    pub vftable: Vftable,
    pub path: StdWstring,
    pub filter_fn: RawPtr,
}

impl IFileDevice for DiskFileDevice {
    fn get_file(
        &self,
        proc: &ProcessRef,
        fs: &FileSystem,
        path: &str,
    ) -> io::Result<Option<Vec<u8>>> {
        let device_path = self.path.read(proc)?;
        let device_path = if device_path.contains(r"\\:") {
            device_path
        } else {
            let cwd = fs.default_device.read(proc)?.path.read(proc)?;
            format!(r"{cwd}\{device_path}")
        };
        #[cfg(windows)]
        let full_path = format!(r"{device_path}\{}", path.replace('/', r"\"));
        #[cfg(target_os = "linux")]
        let full_path = {
            let steam_path = proc.steam_compat_data_path();
            let mut device_path = device_path.replace(r"\", "/");
            if !device_path.chars().next().is_some_and(|ch| ch.is_ascii()) {
                // prevent an unlikely utf boundary panic ig
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "device path is not ASCII",
                ));
            }
            // proton/wine drive letters seem to be lowercase
            device_path[..1].make_ascii_lowercase();
            format!("{steam_path}/pfx/dosdevices/{device_path}/{path}")
        };
        match std::fs::read(full_path) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
}

pub trait IFileDevice {
    fn get_file(
        &self,
        proc: &ProcessRef,
        fs: &FileSystem,
        path: &str,
    ) -> io::Result<Option<Vec<u8>>>;
}

macro_rules! define_subclasses {
    ($name:ident: $iface:ident {$($rtti_name:expr => $impl_type:ident)*}) => {

        #[derive(Debug, Clone)]
        pub enum $name {
            $($impl_type($impl_type),)*
        }

        impl $name {
            /// Praying this gets devirtualized ¯\_(ツ)_/¯
            #[inline]
            pub fn as_dyn(&self) -> &dyn $iface {
                match self {
                    $($name::$impl_type(x) => x as &dyn $iface,)*
                }
            }

            pub fn get(proc: &$crate::memory::ProcessRef, ptr: $crate::memory::RawPtr) -> ::std::io::Result<::std::option::Option<$name>> {
                let vftable = ptr.read::<$crate::memory::Vftable>(proc)?;
                Ok(match vftable.get_rtti_name(proc)?.as_ref() {
                    $(
                        $rtti_name => ::std::option::Option::Some($name::$impl_type(
                            ptr.read::<$impl_type>(proc)?,
                        )),
                    )*
                    x => {
                        ::tracing::warn!("Unknown RTTI name: {x:?}");
                        ::std::option::Option::None
                    },
                })
            }
        }
    };
}

define_subclasses!(FileDevice: IFileDevice {
    ".?AVModDiskFileDeviceCaching@@" => ModDiskFileDeviceCaching
    ".?AVModDiskFileDevice@@" => ModDiskFileDevice
    ".?AVWizardPakFileDevice@@" => WizardPakFileDevice
    ".?AVDiskFileDevice@poro@@" => DiskFileDevice
});
