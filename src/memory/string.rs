use super::*;

#[derive(Clone, Copy, PtrReadable)]
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
            DecodedStdString::Heap(RawPtr::of(u32::read_from_prefix(&self.buf).unwrap().0))
        }
    }
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
            DecodedStdString::Heap(ptr) if ptr.is_null() => {
                write!(f, "heap:\"\"(len={})", self.len)
            }
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

#[derive(FromBytes, IntoBytes, Clone, Copy)]
#[repr(C)]
pub struct StdWstring {
    buf: [u16; 8],
    len: u32,
    cap: u32,
}

#[derive(Clone, Copy)]
pub enum DecodedStdWstring<'a> {
    Inline(&'a [u16]),
    Heap(RawPtr),
}

impl StdWstring {
    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn decode(&self) -> DecodedStdWstring {
        if let Some(inline) = self.buf[..7].get(..self.len as usize) {
            DecodedStdWstring::Inline(inline)
        } else {
            DecodedStdWstring::Heap(RawPtr::of((self.buf[1] as u32) << 16 | self.buf[0] as u32))
        }
    }
}

impl Debug for StdWstring {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(s) =
            DEBUG_PROCESS.with_borrow(|proc| proc.as_ref().and_then(|h| self.read(h).ok()))
        {
            return Debug::fmt(&s, f);
        }

        match self.decode() {
            DecodedStdWstring::Inline(s) => match String::from_utf16(s) {
                Ok(str) => write!(f, "inline:{str:?}"),
                Err(_) => write!(f, "inline:{s:?}"),
            },
            DecodedStdWstring::Heap(ptr) if ptr.is_null() => {
                write!(f, "heap:\"\"(len={})", self.len)
            }
            DecodedStdWstring::Heap(ptr) if self.len != 0 => write!(f, "heap:{ptr:?}"),
            _ => write!(f, "heap:\"\""),
        }
    }
}

impl MemoryStorage for StdWstring {
    type Value = String;

    fn read(&self, proc: &ProcessRef) -> io::Result<Self::Value> {
        match self.decode() {
            DecodedStdWstring::Inline(b) => {
                String::from_utf16(b).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            }
            DecodedStdWstring::Heap(ptr) => match self.len {
                0 => Ok(String::new()),
                _ => String::from_utf16(&proc.read_multiple(ptr.addr(), self.len)?)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            },
        }
    }
}

#[derive(FromBytes, IntoBytes, Clone, Copy)]
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
