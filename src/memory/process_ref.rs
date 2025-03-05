use std::{io, sync::Arc};
use zerocopy::{FromBytes, IntoBytes};

use super::exe_image::PeHeader;

#[derive(Debug, Clone)]
pub struct ProcessRef {
    handle: platform::Handle,
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

    pub fn header(&self) -> Arc<PeHeader> {
        // The only path where this is None is PeHeader::read for obvious reasons
        self.pe_header.as_ref().unwrap().clone()
    }

    pub fn pid(&self) -> u32 {
        self.handle.pid()
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

        pub fn pid(&self) -> u32 {
            self.pid as _
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
        Diagnostics::Debug::ReadProcessMemory, Threading::PROCESS_VM_READ,
    };

    mod threadsafe_handle {
        use std::ops::Deref;
        use windows::{core::Owned, Win32::Foundation::HANDLE};

        /// I'm pretty sure the kernel does not care which thread calls
        /// ReadProcessMemory, as long as it's from the same process.
        ///
        /// > All handles you obtain from functions in Kernel32 are thread-safe,
        /// > unless the MSDN Library article for the function explicitly mentions
        /// > it is not. There's an easy way to tell from your code, such a handle
        /// > is closed with CloseHandle().
        ///
        /// from this one guy on https://stackoverflow.com/a/12214212
        ///
        /// Handle is an opaque number, it's just that in Windows Rust API they
        /// made it !Send and !Sync because it is indeed not always threadsafe
        /// I think?.
        #[derive(Debug)]
        pub struct ThreadsafeHandle(Owned<HANDLE>);
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
        handle: Arc<ThreadsafeHandle>,
    }

    /// Only difference from io::Error::from_os_error (which is the default Into
    /// conversion) is that Rust formats the error as a signed decimal number,
    /// which makes windows error codes into ugly large negatives instead of hex
    /// strings that windows does
    fn better_message(e: windows::core::Error) -> io::Error {
        io::Error::new(io::ErrorKind::Other, e.to_string())
    }

    impl Handle {
        pub fn connect(pid: u32) -> io::Result<Self> {
            Ok(Self {
                pid,
                handle: Arc::new(open_process(PROCESS_VM_READ, pid).map_err(better_message)?),
            })
        }

        pub fn pid(&self) -> u32 {
            self.pid
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

    #[cfg(not(feature = "sneaky"))]
    fn open_process(
        access: windows::Win32::System::Threading::PROCESS_ACCESS_RIGHTS,
        pid: u32,
    ) -> windows::core::Result<ThreadsafeHandle> {
        use windows::Win32::System::Threading::OpenProcess;

        unsafe { OpenProcess(access, false, pid).map(|h| ThreadsafeHandle::new(h)) }
    }

    #[cfg(feature = "sneaky")]
    use direct_syscall::open_process;

    /// A quick and dirty copypaste of the "Hells Gate" technique
    /// (https://fluxsec.red/rust-edr-evasion-hells-gate)
    /// to hopefully stop Windows Defender from being annoying
    /// by *LITERALLY* employing detection evasion rofl
    ///
    /// The idea is that we don't link to NtOpenProcess (which is sus) and
    /// instead do a complicated pointer dance to figure out it's address
    /// without linking to other sus things (thankfully that one is handled by
    /// `export_resolver` crate from the author of the blog post)
    /// and then from that we get the syscall number (which are not static) for
    /// the NtOpenProcess syscall, which we proceed to manually invoke with
    /// (more) inline assembly.
    #[cfg(feature = "sneaky")]
    mod direct_syscall {
        use std::arch::asm;
        use str_crypter::{decrypt_string, sc};
        use windows::{
            Wdk::Foundation::OBJECT_ATTRIBUTES,
            Win32::{
                Foundation::{HANDLE, NTSTATUS},
                System::Threading::PROCESS_ACCESS_RIGHTS,
                System::WindowsProgramming::CLIENT_ID,
            },
        };

        use super::ThreadsafeHandle;

        pub fn open_process(
            access: PROCESS_ACCESS_RIGHTS,
            pid: u32,
        ) -> windows::core::Result<ThreadsafeHandle> {
            let mut process_handle = HANDLE::default();

            let mut object_attributes = OBJECT_ATTRIBUTES {
                Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
                ..Default::default() // zeroed
            };
            let mut client_id = CLIENT_ID {
                UniqueProcess: HANDLE(pid as _),
                ..Default::default() // zeroed
            };

            let status = unsafe {
                nt_open_process(
                    &mut process_handle, // out
                    access.0,
                    &mut object_attributes,
                    &mut client_id, // contains the pid
                )
            };
            if status.is_ok() {
                Ok(unsafe { ThreadsafeHandle::new(process_handle) })
            } else {
                Err(windows::core::Error::from_win32())
            }
        }

        static SSN: std::sync::LazyLock<Option<u32>> = std::sync::LazyLock::new(|| {
            let mut exports = export_resolver::ExportList::new();

            // don't put sus strings in .rdata (also result returned can never be Err)
            let op = sc!("NtOpenProcess", 42).unwrap();
            let dll = sc!("ntdll.dll", 42).unwrap();

            // the api is a bit weird for like caching purposes I suppose
            exports.add(&dll, &op).ok()?;
            // never fails if above succeeded
            let f = exports.get_function_address(&op).unwrap();

            let ssn: u16 = unsafe { *(f as *const u8).add(4).cast() };

            if ssn != 38 {
                tracing::warn!(ssn, "Weird SSN, on last Windows versions it should be 38");
            }

            Some(ssn as _)
        });

        unsafe fn nt_open_process(
            process_handle: *mut HANDLE,
            desired_access: u32,
            object_attributes: *mut OBJECT_ATTRIBUTES,
            client_id: *mut CLIENT_ID,
        ) -> NTSTATUS {
            let ssn = SSN.expect("SSN not found");

            let status: i32;
            asm!(
                "mov r10, rcx",
                "mov eax, {0:e}", // move the syscall number into EAX
                "syscall",
                in(reg) ssn, // input: Syscall number goes into EAX
                // Order: https://web.archive.org/web/20170222171451/https://msdn.microsoft.com/en-us/library/9z1stfyw.aspx
                in("rcx") process_handle,   // passed to RCX (first argument)
                in("rdx") desired_access,   // passed to RDX (second argument)
                in("r8") object_attributes, // passed to R8 (third argument)
                in("r9") client_id,         // passed to R9 (fourth argument)
                lateout("rax") status,      // output: returned value of the syscall is placed in RAX
                options(nostack),           // dont modify the stack pointer
            );
            NTSTATUS(status)
        }
    }
}
