use std::{io, sync::Arc};
use zerocopy::{FromBytes, IntoBytes};

use super::exe_image::PeHeader;

/// A reference to a process, can be cheaply cloned.
#[derive(Debug, Clone)]
pub struct ProcessRef {
    handle: platform::Handle,
    // Used for the timestamp in structs that changed between versions
    pe_header: Option<Arc<PeHeader>>,
}

impl PartialEq for ProcessRef {
    fn eq(&self, other: &Self) -> bool {
        self.handle.pid() == other.handle.pid()
    }
}
impl Eq for ProcessRef {}

impl ProcessRef {
    pub fn connect(pid: u32) -> io::Result<Self> {
        let mut proc = Self {
            handle: platform::Handle::connect(pid)?,
            pe_header: None,
        };
        let pe_header = PeHeader::read(&proc).map_err(io::Error::other)?; // eh just wrap it into io::other for now
        proc.pe_header = Some(Arc::new(pe_header));
        Ok(proc)
    }

    pub fn header(&self) -> &PeHeader {
        // The only path where this is None is PeHeader::read for obvious reasons
        self.pe_header.as_ref().unwrap()
    }

    pub const fn pid(&self) -> u32 {
        self.handle.pid()
    }

    pub const fn base(&self) -> usize {
        self.handle.base()
    }

    #[cfg(target_os = "linux")]
    pub fn steam_compat_data_path(&self) -> &str {
        self.handle.steam_compat_data_path()
    }

    pub fn read_multiple<T: Pod>(&self, addr: u32, len: u32) -> io::Result<Vec<T>> {
        let mut v = T::new_vec_zeroed(len as usize).expect("alloc error");
        self.handle.read_memory(addr as usize, v.as_mut_bytes())?;
        Ok(v)
    }

    pub fn read<T: Pod>(&self, addr: u32) -> io::Result<T> {
        let mut t = T::new_zeroed();
        self.handle.read_memory(addr as usize, t.as_mut_bytes())?;
        Ok(t)
    }
}

/// A shortcut for the zerocopy traits and sanity bounds
pub trait Pod: IntoBytes + FromBytes + Sized + 'static {}

/// Allows us to auto-implement Pod too
impl<T: IntoBytes + FromBytes + Sized + 'static> Pod for T {}

#[cfg(target_os = "linux")]
mod platform {
    use libc::{c_void, iovec, process_vm_readv};
    use std::{io, sync::Arc};

    #[derive(Debug, Clone)]
    pub struct Handle {
        pid: libc::pid_t,
        steam_compat_data_path: Arc<str>,
    }

    impl Handle {
        pub fn connect(pid: u32) -> io::Result<Self> {
            let env = std::fs::read_to_string(format!("/proc/{pid}/environ"))?;
            let steam_compat_data_path = env
                .split('\0')
                .find_map(|s| s.strip_prefix("STEAM_COMPAT_DATA_PATH="))
                .unwrap_or_default()
                .into();
            Ok(Self {
                pid: pid as libc::pid_t,
                steam_compat_data_path,
            })
        }

        pub fn steam_compat_data_path(&self) -> &str {
            &self.steam_compat_data_path
        }

        pub const fn pid(&self) -> u32 {
            self.pid as _
        }

        pub const fn base(&self) -> usize {
            0x0040_0000
        }

        pub fn read_memory(&self, addr: usize, buf: &mut [u8]) -> io::Result<()> {
            if buf.is_empty() {
                return Ok(());
            }
            let local_iov = iovec {
                iov_base: buf.as_mut_ptr() as *mut c_void,
                iov_len: buf.len(),
            };
            let remote_iov = iovec {
                iov_base: addr as *mut c_void,
                iov_len: buf.len(),
            };
            let result = unsafe { process_vm_readv(self.pid, &local_iov, 1, &remote_iov, 1, 0) };
            if result == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::{io, sync::Arc};
    use windows::Win32::System::{
        Diagnostics::Debug::ReadProcessMemory,
        ProcessStatus::EnumProcessModules,
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    };

    mod threadsafe_handle {
        use std::ops::Deref;
        use windows::{Win32::Foundation::HANDLE, core::Owned};

        /// I'm pretty sure the kernel does not care which thread calls
        /// ReadProcessMemory, as long as it's from the same process.
        ///
        /// > All handles you obtain from functions in Kernel32 are thread-safe,
        /// > unless the MSDN Library article for the function explicitly mentions
        /// > it is not. There's an easy way to tell from your code, such a handle
        /// > is closed with CloseHandle().
        ///
        /// from this one guy on https://stackoverflow.com/a/12214212
        #[derive(Debug)]
        pub struct ThreadsafeHandle(Owned<HANDLE>);

        // Handle is an opaque number, it's just that in Windows Rust API they
        // made it !Send and !Sync because it is indeed not always threadsafe
        // I think?.
        unsafe impl Send for ThreadsafeHandle {}
        unsafe impl Sync for ThreadsafeHandle {}

        impl ThreadsafeHandle {
            /// SAFETY:
            /// Up to the caller to determine if the given handle is owned and
            /// indeed threadsafe lol
            pub unsafe fn new(handle: HANDLE) -> Self {
                Self(unsafe { Owned::new(handle) })
            }
        }

        impl Deref for ThreadsafeHandle {
            type Target = HANDLE;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    }
    use threadsafe_handle::ThreadsafeHandle;

    #[derive(Debug, Clone)]
    pub struct Handle {
        pid: u32,
        base: usize,
        handle: Arc<ThreadsafeHandle>,
    }

    /// Only difference from io::Error::from_os_error (which is the default Into
    /// conversion) is that Rust formats the error as a signed decimal number,
    /// which makes windows error codes into ugly large negatives instead of hex
    /// strings that windows does
    fn better_message(e: windows::core::Error) -> io::Error {
        io::Error::other(e.to_string())
    }

    impl Handle {
        pub fn connect(pid: u32) -> io::Result<Self> {
            let handle =
                unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) }
                    .map(|h| unsafe { ThreadsafeHandle::new(h) })
                    .map_err(better_message)?;

            let mut module = unsafe { std::mem::zeroed() };
            let mut cb_needed = 0;
            unsafe {
                EnumProcessModules(
                    *handle,
                    &mut module,
                    std::mem::size_of_val(&module) as _,
                    &mut cb_needed,
                )
            }?;

            Ok(Self {
                pid,
                base: module.0 as _,
                handle: Arc::new(handle),
            })
        }

        pub const fn pid(&self) -> u32 {
            self.pid
        }

        pub const fn base(&self) -> usize {
            self.base
        }

        pub fn read_memory(&self, addr: usize, buf: &mut [u8]) -> io::Result<()> {
            if buf.is_empty() {
                return Ok(());
            }

            unsafe {
                ReadProcessMemory(
                    **self.handle,
                    addr as _,
                    buf.as_mut_ptr() as _,
                    buf.len(),
                    None,
                )
            }
            .map_err(better_message)?;
            Ok(())
        }
    }
}
