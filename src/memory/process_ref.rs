use std::{
    fmt::{self, Debug},
    io,
};

use read_process_memory::{CopyAddress, Pid, ProcessHandle};
use zerocopy::{AsBytes, FromBytes};

/// A a bit of a nicer wrapper over `read_process_memory` crate API.
/// Could reimplement it/swap it here.
#[derive(Clone)]
pub struct ProcessRef {
    /// pid is used for debug and equality
    pid: u32,
    /// On Linux handle is just the pid again, but on win it's a HANDLE
    /// The win handle is in an Arc so it's cheap to clone around
    handle: ProcessHandle,
}

impl PartialEq for ProcessRef {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}
impl Eq for ProcessRef {}

impl Debug for ProcessRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ProcessRef").field(&self.pid).finish()
    }
}

impl ProcessRef {
    pub fn connect(pid: u32) -> io::Result<Self> {
        Ok(Self {
            pid,
            handle: (pid as Pid).try_into()?,
        })
    }

    pub fn read_multiple<T: Pod>(&self, addr: u32, len: u32) -> io::Result<Vec<T>> {
        let mut v = T::new_vec_zeroed(len as usize);
        self.handle.copy_address(addr as usize, v.as_bytes_mut())?;
        Ok(v)
    }

    pub fn read<T: Pod>(&self, addr: u32) -> io::Result<T> {
        let mut t = T::new_zeroed();
        self.handle.copy_address(addr as usize, t.as_bytes_mut())?;
        Ok(t)
    }
}

impl TryFrom<u32> for ProcessRef {
    type Error = io::Error;

    fn try_from(pid: u32) -> Result<Self, Self::Error> {
        Self::connect(pid)
    }
}

/// A shortcut for the zerocopy traits and sanity bounds
pub trait Pod: AsBytes + FromBytes + Sized + 'static {}

impl<T: AsBytes + FromBytes + Sized + 'static> Pod for T {}
