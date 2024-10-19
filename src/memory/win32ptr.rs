use core::fmt;
use std::{fmt::Debug, io, marker::PhantomData, mem::size_of};

use zerocopy::{FromBytes, IntoBytes};

use crate::memory::debug_type;

use super::{process_ref::Pod, MemoryStorage, ProcessRef};

#[derive(FromBytes, IntoBytes, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct RawPtr(u32);

impl RawPtr {
    pub const fn of(addr: u32) -> Self {
        Self(addr)
    }

    pub const fn cast<T>(self) -> Ptr<T> {
        Ptr::of(self.0)
    }

    pub const fn addr(self) -> u32 {
        self.0
    }

    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    pub fn read_multiple<T: Pod>(self, proc: &ProcessRef, len: u32) -> io::Result<Vec<T>> {
        proc.read_multiple(self.0, len)
    }

    pub fn read_at<T: Pod>(self, offset: u32, proc: &ProcessRef) -> io::Result<T> {
        proc.read(self.0 + offset)
    }

    pub fn read<T: Pod>(self, proc: &ProcessRef) -> io::Result<T> {
        proc.read(self.0)
    }
}

impl Debug for RawPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08x}", self.0)
    }
}

impl From<u32> for RawPtr {
    fn from(addr: u32) -> Self {
        Self::of(addr)
    }
}

#[derive(FromBytes, IntoBytes)]
#[repr(transparent)]
pub struct Ptr<T, const BASE: u32 = 0> {
    raw: RawPtr,
    _phantom: PhantomData<T>,
}

pub type Ibo<T> = Ptr<T, 0x0040_0000>;

impl<T, const BASE: u32> Ptr<T, BASE> {
    pub const fn of(addr: u32) -> Self {
        Self {
            raw: RawPtr::of(addr),
            _phantom: PhantomData,
        }
    }

    pub const fn offset(self, offset: i32) -> Self {
        Self::of((self.raw.addr() as i32 + offset * size_of::<T>() as i32) as u32)
    }
}

impl<T> Ptr<T> {
    pub const fn addr(self) -> u32 {
        self.raw.addr()
    }

    pub const fn is_null(&self) -> bool {
        self.addr() == 0
    }

    pub const fn raw(self) -> RawPtr {
        RawPtr::of(self.addr())
    }
}

impl<T, const BASE: u32> Clone for Ptr<T, BASE> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, const BASE: u32> Copy for Ptr<T, BASE> {}

impl<T, const BASE: u32> PartialEq for Ptr<T, BASE> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T, const BASE: u32> Eq for Ptr<T, BASE> {}

impl<T, const BASE: u32> Debug for Ptr<T, BASE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if BASE == 0 {
            if self.raw.is_null() {
                write!(f, "NULL")
            } else {
                write!(f, "{:?} as {}", self.raw, debug_type::<T>())
            }
        } else {
            write!(f, "0x{BASE:08x}+{:?} as {}", self.raw, debug_type::<T>())
        }
    }
}

impl<T, const BASE: u32> From<u32> for Ptr<T, BASE> {
    fn from(addr: u32) -> Self {
        Self::of(addr)
    }
}

impl<T: Pod, const BASE: u32> MemoryStorage for Ptr<T, BASE> {
    type Value = T;

    fn read(&self, proc: &ProcessRef) -> io::Result<Self::Value> {
        self.raw.read_at(BASE, proc)
    }
}

// Sadly, this is a specialization, for it to work we need a blanket noop impl
// for MemoryStorage, which would conflict with this
//
// impl<T: MemoryStorage, const BASE: u32> MemoryStorage for Ptr<T, BASE> {
//     type Value = T::Value;

//     fn read(&self, proc: &ProcessRef) -> io::Result<Self::Value> {
//         self.raw.read_at::<T>(BASE, proc)?.read(proc)
//     }
// }
