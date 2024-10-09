use std::{
    any::type_name,
    borrow::{Borrow, Cow},
    cell::RefCell,
    cmp::Ordering,
    fmt::{self, Debug, Display},
    io,
};

use lazy_regex::regex_replace_all;
use zerocopy::{AsBytes, FromBytes, FromZeroes};

mod process_ref;
mod win32ptr;

pub mod exe_image;

pub use process_ref::{Pod, ProcessRef};
pub use win32ptr::{Ibo, Ptr, RawPtr};

#[derive(AsBytes, FromBytes, FromZeroes, Clone, Copy)]
#[repr(C, packed)]
pub struct PadBool<const PAD: usize = 0>(u8, [u8; PAD]);

pub type ByteBool = PadBool<0>;

impl<const PAD: usize> PadBool<PAD> {
    pub fn as_bool(&self) -> bool {
        self.0 != 0
    }
}

impl<const PAD: usize> Debug for PadBool<PAD> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            0 => write!(f, "false"),
            1 => write!(f, "true"),
            x => write!(f, "ByteBool({x}, {:?})", { self.1 }),
        }
    }
}

impl<const PAD: usize> Display for PadBool<PAD> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.as_bool(), f)
    }
}

impl<const PAD: usize> From<bool> for PadBool<PAD> {
    fn from(b: bool) -> Self {
        Self(b as u8, [0; PAD])
    }
}

impl<const PAD: usize> From<PadBool<PAD>> for bool {
    fn from(b: PadBool<PAD>) -> Self {
        b.as_bool()
    }
}

// A hack to make zerocopy shut up
#[derive(AsBytes, FromBytes, FromZeroes)]
#[repr(transparent)]
pub struct RealignedF64([u32; 2]);

impl RealignedF64 {
    pub fn as_f64(&self) -> f64 {
        f64::from_bits(self.0[0] as u64 | (self.0[1] as u64) << 32)
    }
}

impl Debug for RealignedF64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.as_f64(), f)
    }
}

impl Display for RealignedF64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.as_f64(), f)
    }
}

impl From<f64> for RealignedF64 {
    fn from(f: f64) -> Self {
        let bits = f.to_bits();
        Self([bits as u32, (bits >> 32) as u32])
    }
}

impl From<RealignedF64> for f64 {
    fn from(f: RealignedF64) -> Self {
        f.as_f64()
    }
}

pub trait MemoryStorage: Pod {
    type Value;

    fn read(&self, proc: &ProcessRef) -> io::Result<Self::Value>;

    fn bind(self, proc: ProcessRef) -> Remote<Self>
    where
        Self: Sized,
    {
        Remote::new(proc, self)
    }
}

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
        )*
    };
}

primitives!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

#[derive(AsBytes, FromBytes, FromZeroes, Clone, Copy)]
#[repr(C)]
pub struct StdString {
    buf: [u8; 16],
    len: u32,
    cap: u32,
}

#[derive(Clone, Copy)]
pub enum DecodedStdString<'a> {
    Inline(&'a [u8]),
    Heap(RawPtr),
}

impl StdString {
    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn decode(&self) -> DecodedStdString {
        if let Some(inline) = self.buf[..15].get(..self.len as usize) {
            DecodedStdString::Inline(inline)
        } else {
            DecodedStdString::Heap(RawPtr::of(u32::read_from_prefix(&self.buf).unwrap()))
        }
    }
}

// thread local because on win Handle is not Sync
// and a RefCell because it's not Copy
thread_local! {
    static DEBUG_PROCESS: RefCell<Option<ProcessRef>> = const { RefCell::new(None) };
}
pub fn set_debug_process(proc: ProcessRef) {
    DEBUG_PROCESS.set(Some(proc));
}

impl Debug for StdString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(s) =
            DEBUG_PROCESS.with_borrow(|proc| proc.as_ref().and_then(|h| self.read(h).ok()))
        {
            return Debug::fmt(&s, f);
        }

        match self.decode() {
            DecodedStdString::Inline(s) => match std::str::from_utf8(s) {
                Ok(str) => write!(f, "inline:{str:?}"),
                Err(_) => write!(f, "inline:{s:?}"),
            },
            DecodedStdString::Heap(ptr) if self.len != 0 => write!(f, "heap:{ptr:?}"),
            _ => write!(f, "heap:\"\""),
        }
    }
}

impl MemoryStorage for StdString {
    type Value = String;

    fn read(&self, proc: &ProcessRef) -> io::Result<Self::Value> {
        match self.decode() {
            DecodedStdString::Inline(b) => std::str::from_utf8(b)
                .map(|s| s.to_owned()) // lifetimes are super fun and cool and dandy if you try to have Cow here lul
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            DecodedStdString::Heap(ptr) => {
                if self.len == 0 {
                    return Ok(String::new());
                }
                String::from_utf8(proc.read_multiple(ptr.addr(), self.len)?)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            }
        }
    }
}

#[derive(AsBytes, FromBytes, FromZeroes, Clone, Copy)]
#[repr(transparent)]
pub struct CString(RawPtr);

impl CString {
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl Debug for CString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(s) =
            DEBUG_PROCESS.with_borrow(|proc| proc.as_ref().and_then(|h| self.read(h).ok()))
        {
            return Debug::fmt(&s, f);
        }

        f.debug_tuple("CString").field(&self.0).finish()
    }
}

impl From<CString> for RawPtr {
    fn from(c: CString) -> Self {
        c.0
    }
}

impl From<RawPtr> for CString {
    fn from(p: RawPtr) -> Self {
        Self(p)
    }
}

impl MemoryStorage for CString {
    type Value = String;

    fn read(&self, proc: &ProcessRef) -> io::Result<Self::Value> {
        let mut size = 64; // idk seems reasonable we'll very rarely hit the doubling even once

        while size != 2048 {
            let mut buf = self.0.read_multiple(proc, size)?;
            if let Some(len) = buf.iter().position(|&b| b == 0) {
                buf.truncate(len);
                return String::from_utf8(buf)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
            }
            size *= 2;
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("CString too long (at {:?})", self.0),
        ))
    }
}

#[derive(AsBytes, FromBytes, FromZeroes)]
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
        T: Pod,
    {
        self.get(index).map(|p| p.read(proc)).transpose()
    }
}

impl<T> Debug for StdVec<T>
where
    T: Pod + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(s) =
            DEBUG_PROCESS.with_borrow(|proc| proc.as_ref().and_then(|h| self.read(h).ok()))
        {
            return Debug::fmt(&s, f);
        }
        write!(f, "StdVec[{} * {}]", self.len(), debug_type::<T>())
    }
}

impl<T: MemoryStorage> StdVec<T> {
    pub fn read_storage(&self, proc: &ProcessRef) -> io::Result<Vec<T::Value>> {
        let len = self.len();
        let mut vec = Vec::with_capacity(len as usize);
        for i in 0..len {
            vec.push(self.get(i).unwrap().read(proc)?.read(proc)?);
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

#[derive(Debug, AsBytes, FromBytes, FromZeroes)]
#[repr(C, packed)]
pub struct StdMapNode<K, V> {
    left: Ptr<StdMapNode<K, V>>,
    parent: Ptr<StdMapNode<K, V>>,
    right: Ptr<StdMapNode<K, V>>,
    _meta: u32, // color+pad or smth
    key: K,
    value: V,
}

#[derive(AsBytes, FromBytes, FromZeroes, Clone, Copy)]
#[repr(C, packed)]
pub struct StdMap<K, V> {
    root: Ptr<StdMapNode<K, V>>,
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

impl<K, V> Debug for StdMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // todo when we learn to iterate over the tree, impl complete debug
        write!(
            f,
            "StdMap[{} * ({} => {})]",
            self.len(),
            debug_type::<K>(),
            debug_type::<V>()
        )
    }
}

// why did I have to overengineer this pos lolol
// the whole MemoryStorage thing only exists because of this
impl<K: MemoryStorage, V: MemoryStorage> StdMap<K, V> {
    pub fn get<Q>(&self, proc: &ProcessRef, key: &Q) -> io::Result<Option<V::Value>>
    where
        Q: Ord + ?Sized,
        K::Value: Borrow<Q>,
    {
        let root_ptr = self.root;
        let root = root_ptr.read(proc)?;

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
                Ordering::Equal => return Ok(Some({ node.value }.read(proc)?)),
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

#[derive(Debug)]
pub struct Remote<T> {
    proc: ProcessRef,
    thing: T,
}

impl<T> Remote<T> {
    pub const fn new(proc: ProcessRef, thing: T) -> Self {
        Self { proc, thing }
    }
}

impl<T: MemoryStorage> Remote<T> {
    pub fn read(&self) -> io::Result<T::Value> {
        self.thing.read(&self.proc)
    }
}

pub type RemotePtr<T> = Remote<Ptr<T>>;

pub(crate) fn debug_type<T>() -> Cow<'static, str> {
    regex_replace_all!(r"(?:\w+::)+", type_name::<T>(), "")
}
