//! The profiler that is built into the game.
//!
//! noita.exe has a small profiler, `SimpleProfilerGlobal`, that is always on.
//! Each `ComponentUpdator<XComponent, XSystem>` times its update with it, on
//! the main thread only, under the RTTI name of its component (for example
//! `class LuaComponent`). Those updators are the only users of the profiler,
//! so the map has only `class XComponent` names. A name appears in the map
//! when its system first runs.
//!
//! See `SimpleProfilerGlobal::ScopeEnd`, `SimpleProfilerGlobal::GetOrCreateData`
//! and `SimpleProfilerGlobal::TProfilerData::AddSample` in Ghidra.

use crate::memory::{Ptr, PtrReadable, Raw, StdMap, StdString, Vftable};

/// The first fields of `SimpleProfilerGlobal`.
#[derive(Debug, PtrReadable)]
#[repr(C)]
pub struct SimpleProfilerGlobal {
    /// The data of each scope, by the name of the scope.
    pub data: StdMap<StdString, Raw<Ptr<ProfilerData>>>,
    /// The thread that the profiler records samples for.
    pub thread_id: u64,
}

/// `SimpleProfilerGlobal::TProfilerData`, the samples of one scope.
#[derive(Debug, Clone, Copy, PtrReadable)]
#[repr(C)]
pub struct ProfilerData {
    pub vftable: Vftable,
    _pad: u32,
    /// The sum of all the samples, in milliseconds.
    pub total_ms: f64,
    /// The last sample, in milliseconds.
    pub last_ms: f64,
    /// The number of samples.
    pub count: u32,
    _pad2: u32,
    pub min_ms: f64,
    pub max_ms: f64,
    /// `ScopeEnd` sets the first dword from the scope, when it is not 0.
    _field_0x30: [u32; 2],
    /// The last [`PROFILER_HISTORY`] samples, a ring buffer.
    pub history_ms: [f64; PROFILER_HISTORY],
    /// The index in `history_ms` of the next sample.
    pub history_index: u32,
    _field_0x12c: u32,
}
const _: () = assert!(std::mem::size_of::<ProfilerData>() == 0x130);

/// The number of samples in [`ProfilerData::history_ms`].
pub const PROFILER_HISTORY: usize = 30;
