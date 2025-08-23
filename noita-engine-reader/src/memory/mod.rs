use std::{
    any::type_name,
    borrow::{Borrow, Cow},
    cell::RefCell,
    cmp::Ordering,
    collections::HashMap,
    fmt::{self, Debug, Display},
    hash::Hash,
    io,
};

use lazy_regex::regex_replace_all;
use serde::{Serialize, Serializer};
use zerocopy::{FromBytes, IntoBytes};

mod process_ref;
mod string;
mod win32ptr;

pub mod exe_image;

pub use process_ref::*;
pub use string::*;
pub use win32ptr::*;

pub use noita_engine_reader_macros::PtrReadable;

#[derive(FromBytes, IntoBytes, Clone, Copy)]
#[repr(C, packed)]
pub struct WithPad<T: Copy, const PAD: usize = 0>(T, [u8; PAD]);

impl<T: Copy, const PAD: usize> WithPad<T, PAD> {
    pub fn get(&self) -> T {
        self.0
    }
}

impl<T: Copy + Debug, const PAD: usize> Debug for WithPad<T, PAD> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "{:?}+{:02x?}", self.get(), { self.1 })
        } else {
            Debug::fmt(&self.get(), f)
        }
    }
}

impl<T: Copy + Display, const PAD: usize> Display for WithPad<T, PAD> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.get(), f)
    }
}

impl<T: Copy, const PAD: usize> From<T> for WithPad<T, PAD> {
    fn from(t: T) -> Self {
        Self(t, [0; PAD])
    }
}

#[derive(FromBytes, IntoBytes, Clone, Copy)]
#[repr(transparent)]
pub struct ByteBool(u8);

pub type PadBool<const PAD: usize> = WithPad<ByteBool, PAD>;

impl<const PAD: usize> PadBool<PAD> {
    pub fn as_bool(&self) -> bool {
        self.0.as_bool()
    }
}

impl<T: Serialize + Copy, const PAD: usize> Serialize for WithPad<T, PAD> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.get().serialize(serializer)
    }
}

impl Serialize for ByteBool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(self.as_bool())
    }
}

impl ByteBool {
    pub fn as_bool(&self) -> bool {
        debug_assert!(self.0 == 0 || self.0 == 1, "Invalid boolean: {self:?}");
        self.0 != 0
    }
}

impl Debug for ByteBool {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            0 => f.write_str("false"),
            1 => f.write_str("true"),
            x => write!(f, "ByteBool({x:02x})"),
        }
    }
}

impl Display for ByteBool {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.as_bool(), f)
    }
}

impl From<bool> for ByteBool {
    fn from(b: bool) -> Self {
        Self(b as u8)
    }
}

impl From<ByteBool> for bool {
    fn from(b: ByteBool) -> Self {
        b.as_bool()
    }
}

#[derive(FromBytes, IntoBytes, Clone, Copy)]
#[repr(C, packed(4))]
pub struct Align4<T: Copy>(T);

impl<T: Debug + Copy> Debug for Align4<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&{ self.0 }, f)
    }
}

impl<T: Display + Copy> Display for Align4<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&{ self.0 }, f)
    }
}

impl<T: Copy> Align4<T> {
    pub fn get(self) -> T {
        self.0
    }
}

impl<T: Copy> From<T> for Align4<T> {
    fn from(t: T) -> Self {
        Self(t)
    }
}

pub trait MemoryStorage: Pod {
    type Value;

    fn read(&self, proc: &ProcessRef) -> io::Result<Self::Value>;
}

/// Marker trait for types that can be read from behind a pointer
pub trait PtrReadable: Pod {}

// specialization (where the default is passthrough like this and only few
// select types actually read foreign memory) would've been nice
macro_rules! primitives {
    ($($t:ty),*) => {
        $(
            impl MemoryStorage for $t {
                type Value = Self;

                fn read(&self, _: &ProcessRef) -> io::Result<Self::Value> {
                    Ok(*self)
                }
            }

            impl<const N: usize> MemoryStorage for [$t; N] {
                type Value = Self;

                fn read(&self, _: &ProcessRef) -> io::Result<Self::Value> {
                    Ok(*self)
                }
            }

            impl PtrReadable for $t {}
            impl<const N: usize> PtrReadable for [$t; N] {}
        )*
    };
}

primitives!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

/// An escape hatch for the above lack of specialization
#[derive(FromBytes, IntoBytes, Clone, Copy)]
#[repr(transparent)]
pub struct Raw<T>(T);

impl<T: PtrReadable> PtrReadable for Raw<T> {}

impl<T: Pod + Clone> MemoryStorage for Raw<T> {
    type Value = T;

    fn read(&self, _: &ProcessRef) -> io::Result<Self::Value> {
        Ok(self.0.clone())
    }
}

impl<T: Debug> Debug for Raw<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<T: Serialize> Serialize for Raw<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

// thread local because on win Handle is not Sync
// and a RefCell because it's not Copy
thread_local! {
    pub(crate) static DEBUG_PROCESS: RefCell<Option<ProcessRef>> = const { RefCell::new(None) };
}

pub fn set_debug_process(proc: ProcessRef) {
    DEBUG_PROCESS.set(Some(proc));
}

#[derive(PtrReadable)]
#[repr(C, packed)]
pub struct StdVec<T> {
    start: Ptr<T>,
    end: Ptr<T>,
    cap: Ptr<T>,
}

impl<T> Clone for StdVec<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for StdVec<T> {}

impl<T> StdVec<T> {
    pub fn len(&self) -> u32 {
        self.end.addr().wrapping_sub(self.start.addr()) / size_of::<T>() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.start.addr() == self.end.addr()
    }

    pub fn get(&self, index: u32) -> Option<Ptr<T>> {
        if index < self.len() {
            Some(Ptr::of(self.start.addr() + index * size_of::<T>() as u32))
        } else {
            None
        }
    }

    pub fn truncated(&self, len: u32) -> StdVec<T> {
        if len >= self.len() {
            return *self;
        }
        StdVec {
            start: self.start,
            end: self.get(len).unwrap(), // just checked that len is always in bounds
            cap: self.cap,
        }
    }

    pub fn read_at(&self, index: u32, proc: &ProcessRef) -> io::Result<Option<T>>
    where
        T: PtrReadable,
    {
        self.get(index).map(|p| p.read(proc)).transpose()
    }
}

impl<T> Debug for StdVec<T>
where
    T: Pod + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // an heuristic to avoid printing huge or invalid vectors
        if self.len() < 4096 {
            if let Some(s) =
                DEBUG_PROCESS.with_borrow(|proc| proc.as_ref().and_then(|h| self.read(h).ok()))
            {
                return Debug::fmt(&s, f);
            }
        }
        write!(f, "StdVec[{} * {}]", self.len(), debug_type::<T>())
    }
}

impl<T> Serialize for StdVec<T>
where
    T: MemoryStorage + PtrReadable + Serialize,
    <T as MemoryStorage>::Value: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(vec) =
            DEBUG_PROCESS.with_borrow(|proc| proc.as_ref().and_then(|h| self.read_storage(h).ok()))
        {
            vec.serialize(serializer)
        } else {
            serializer.serialize_none()
        }
    }
}

impl<T: MemoryStorage + PtrReadable> StdVec<T> {
    pub fn read_storage(&self, proc: &ProcessRef) -> io::Result<Vec<T::Value>> {
        let len = self.len();
        let mut vec = Vec::with_capacity(len as usize);
        for i in 0..len {
            vec.push(self.read_at(i, proc)?.unwrap().read(proc)?);
        }
        Ok(vec)
    }
}

impl<T: Pod> MemoryStorage for StdVec<T> {
    type Value = Vec<T>;

    fn read(&self, proc: &ProcessRef) -> io::Result<Self::Value> {
        proc.read_multiple(self.start.addr(), self.len())
    }
}

#[derive(Debug, FromBytes, IntoBytes)]
#[repr(C, packed)]
pub struct StdMapNode<K, V> {
    left: Ptr<StdMapNode<K, V>>,
    parent: Ptr<StdMapNode<K, V>>,
    right: Ptr<StdMapNode<K, V>>,
    _meta: u32, // color+pad or smth
    key: K,
    value: V,
}

impl<K: Pod, V: Pod> PtrReadable for StdMapNode<K, V> {}

#[derive(FromBytes, IntoBytes, Clone)]
#[repr(C, packed)]
pub struct StdMap<K, V> {
    sentinel: Ptr<StdMapNode<K, V>>,
    len: u32,
}

impl<K, V> StdMap<K, V> {
    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<K, V> Debug for StdMap<K, V>
where
    K: MemoryStorage,
    V: MemoryStorage,
    K::Value: Eq + Hash + Debug,
    V::Value: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.len() < 512 {
            if let Some(s) =
                DEBUG_PROCESS.with_borrow(|proc| proc.as_ref().and_then(|h| self.read(h).ok()))
            {
                return Debug::fmt(&s, f);
            }
        }
        write!(
            f,
            "StdMap[{} * ({} => {})]",
            self.len(),
            debug_type::<K>(),
            debug_type::<V>()
        )
    }
}

impl<K, V> MemoryStorage for StdMap<K, V>
where
    K: MemoryStorage,
    K::Value: Eq + Hash,
    V: MemoryStorage,
{
    type Value = HashMap<K::Value, V::Value>;

    fn read(&self, proc: &ProcessRef) -> io::Result<Self::Value> {
        let mut result = HashMap::with_capacity(self.len() as _);
        let root = { self.sentinel }.read(proc)?.parent;

        // just do bfs on the tree ig - this is unordered;
        // for ordered we need to start from sentinel.left/sentinel.right
        // (which are the smallest/biggest nodes) and do the correct
        // red-black tree traversal type of thing
        let mut stack = vec![root];
        while let Some(node_ptr) = stack.pop() {
            if node_ptr == { self.sentinel } || node_ptr.is_null() {
                continue;
            }
            let node = node_ptr.read(proc)?;
            result.insert({ node.key }.read(proc)?, { node.value }.read(proc)?);
            stack.push(node.right);
            stack.push(node.left);
        }
        Ok(result)
    }
}

// why did I have to overengineer this pos lolol
// the whole MemoryStorage thing only exists because of this
impl<K: MemoryStorage, V> StdMap<K, V> {
    pub fn get<Q>(&self, proc: &ProcessRef, key: &Q) -> io::Result<Option<V::Value>>
    where
        V: MemoryStorage,
        Q: Ord + ?Sized,
        K::Value: Borrow<Q>,
    {
        self.get_raw(proc, key)?.map(|v| v.read(proc)).transpose()
    }

    #[track_caller]
    pub fn get_raw<Q>(&self, proc: &ProcessRef, key: &Q) -> io::Result<Option<V>>
    where
        V: Pod,
        Q: Ord + ?Sized,
        K::Value: Borrow<Q>,
    {
        let root_ptr = self.sentinel;
        let root = root_ptr.read(proc)?;

        if { root.parent } == root_ptr || { root.parent }.is_null() {
            return Ok(None);
        }

        // The root node is special, root.parent is the actual root node
        // of the binary tree and root.left/root.right are first/last I think
        let mut node = { root.parent }.read(proc)?;

        // not a loop{} just in case idk, nasa told me to do so
        for _ in 0..100 {
            let node_key = node.key;
            let node_key = node_key.read(proc)?;

            let next = match key.cmp(node_key.borrow()) {
                Ordering::Less => node.left,
                Ordering::Greater => node.right,
                Ordering::Equal => return Ok(Some(node.value)),
            };
            // the root pointer is used as the sentinel (and not just null?. huh)
            if next == root_ptr || next.is_null() {
                return Ok(None);
            }
            node = next.read(proc)?;
        }
        Ok(None)
    }
}

pub(crate) fn debug_type<T>() -> Cow<'static, str> {
    regex_replace_all!(r"(?:\w+::)+", type_name::<T>(), "")
}

#[derive(FromBytes, IntoBytes)]
#[repr(C, packed)]
struct StdUnorderedMapNode<K, V> {
    next: Ptr<StdUnorderedMapNode<K, V>>,
    _unknown: u32, // cached hash?
    key: K,
    value: V,
}
impl<K: Pod, V: Pod> PtrReadable for StdUnorderedMapNode<K, V> {}

#[derive(FromBytes, IntoBytes)]
#[repr(C, packed)]
pub struct StdUnorderedMap<K, V> {
    sentinel: Ptr<StdUnorderedMapNode<K, V>>,
    size: u32,
    buckets: StdVec<Ptr<StdUnorderedMapNode<K, V>>>,
    hash_mask: u32,
    table_size: u32,
    load_factor: f32,
}

impl<K, V> StdUnorderedMap<K, V> {
    pub fn read_keys(&self, proc: &ProcessRef) -> io::Result<Vec<K::Value>>
    where
        K: MemoryStorage,
        V: MemoryStorage,
    {
        let mut res = Vec::new();

        let mut entry = { self.sentinel }.read(proc)?.next;
        while entry != { self.sentinel } {
            let e = entry.read(proc)?;
            let key = { e.key }.read(proc)?;
            res.push(key);
            entry = e.next;
        }

        Ok(res)
    }
}
