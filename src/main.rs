use std::borrow::Cow;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::{cmp::Ordering, fmt::Debug};

use anyhow::{Context, Result};
use bytemuck::AnyBitPattern;
use clap::Parser;
use process_memory::{CopyAddress, Pid, ProcessHandle, TryIntoProcessHandle};
use strfmt::Format;
use sysinfo::ProcessesToUpdate;

#[derive(AnyBitPattern, Clone, Copy)]
#[repr(transparent)]
struct RemotePtr<T> {
    addr: u32,
    _phantom: PhantomData<T>,
}

impl<T> Debug for RemotePtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:x}", self.addr)
    }
}

impl<T> RemotePtr<T> {
    fn new(addr: u32) -> Self {
        Self {
            addr,
            _phantom: PhantomData,
        }
    }

    /// Read size_of::<T> bytes from the process and transmute them into T
    fn read(&self, handle: ProcessHandle) -> Result<T>
    where
        T: AnyBitPattern,
    {
        let mut buf = vec![0; std::mem::size_of::<T>()];

        handle.copy_address(self.addr as usize, &mut buf)?;

        Ok(*bytemuck::from_bytes(&buf))
    }
}

#[derive(AnyBitPattern, Clone, Copy)]
#[repr(C)]
struct StdString {
    buf: [u8; 16],
    len: u32,
    cap: u32,
}

impl StdString {
    fn read(&self, handle: ProcessHandle) -> Result<Cow<str>> {
        if self.len <= 15 {
            let data = &self.buf[..self.len as usize];
            return Ok(Cow::Borrowed(std::str::from_utf8(data)?));
        }

        let ptr = u32::from_le_bytes(self.buf[..4].try_into().unwrap());
        let mut buf = vec![0; self.len as usize];

        handle.copy_address(ptr as usize, &mut buf)?;

        Ok(Cow::Owned(String::from_utf8(buf)?))
    }
}

impl Debug for StdString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.len <= 15 {
            let data = &self.buf[..self.len as usize];

            return match std::str::from_utf8(data) {
                Ok(str) => write!(f, "inline:{str:?}"),
                Err(_) => write!(f, "inline:{data:?}"),
            };
        }
        write!(
            f,
            "heap:0x{:x}",
            u32::from_le_bytes(self.buf[..4].try_into().unwrap())
        )
    }
}

#[derive(Debug, AnyBitPattern, Clone, Copy)]
#[repr(C)]
struct StringIntMapNode {
    left: RemotePtr<StringIntMapNode>,
    parent: RemotePtr<StringIntMapNode>,
    right: RemotePtr<StringIntMapNode>,
    _meta: u32, // color+pad or smth
    key: StdString,
    value: u32,
}

impl RemotePtr<StringIntMapNode> {
    fn get(&self, handle: ProcessHandle, key: &str) -> Result<Option<u32>> {
        let map = self.read(handle)?;

        // The toplevel map node is special, map.parent is the actual root node
        // of the binary tree and map.left/map.right are first/last I think
        let mut node = map.parent.read(handle)?;

        // not a loop{} just in case idk
        for _ in 0..100 {
            let node_key = node.key.read(handle)?;

            let next = match key.cmp(&node_key) {
                Ordering::Less => node.left,
                Ordering::Greater => node.right,
                Ordering::Equal => return Ok(Some(node.value)),
            };
            if next.addr == self.addr {
                return Ok(None);
            }
            node = next.read(handle)?;
        }
        Ok(None)
    }
}

#[derive(clap::Parser)]
struct Args {
    /// Format of the output
    #[arg(
        default_value = "Deaths: {deaths}\nWins:   {wins}\nStreak: {streak}\nRecord: {streak-pb}"
    )]
    format: String,
    /// Do not look for noita.exe process and use the given pid instead
    #[arg(long)]
    pid: Option<u32>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let noita_pid = match args.pid {
        Some(pid) => pid,
        None => {
            let mut system = sysinfo::System::new();
            system.refresh_processes(ProcessesToUpdate::All);

            let mut processes = system.processes_by_exact_name("noita.exe".as_ref());

            processes
                .find(|p| p.thread_kind().is_none())
                .map(|p| p.pid())
                .context("Noita process not found")?
                .as_u32()
        }
    } as Pid;

    let handle = noita_pid.try_into_process_handle()?;

    let death_count_member = RemotePtr::new(0x01206ad8);
    let streak_member = RemotePtr::new(0x0120694c);
    let streak_pb_member = RemotePtr::new(0x01206a14);

    let death_count = death_count_member.read(handle)?;
    let streak = streak_member.read(handle)?;
    let streak_pb = streak_pb_member.read(handle)?;

    let map_ptr = RemotePtr::<RemotePtr<StringIntMapNode>>::new(0x01206938);
    let map = map_ptr.read(handle)?;

    let end0 = map.get(handle, "progress_ending0")?;
    let end1 = map.get(handle, "progress_ending1")?;

    let wins = end0.unwrap_or_default() + end1.unwrap_or_default();

    let formatted = args.format.format(&HashMap::from([
        ("deaths".to_owned(), death_count),
        ("streak".to_owned(), streak),
        ("streak-pb".to_owned(), streak_pb),
        ("wins".to_owned(), wins),
    ]))?;

    println!("{formatted}");

    Ok(())
}
