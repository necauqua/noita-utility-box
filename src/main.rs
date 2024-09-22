use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::{cmp::Ordering, fmt::Debug};

use anyhow::{anyhow, bail, Context, Result};
use bytemuck::AnyBitPattern;
use clap::Parser;
use process_memory::{CopyAddress, Pid, ProcessHandle, TryIntoProcessHandle};
use strfmt::{FmtError, Format};
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

    fn read(&self, handle: ProcessHandle) -> Result<T>
    where
        T: AnyBitPattern,
    {
        let mut buf = vec![0; size_of::<T>()];

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

static DEBUG_HANDLE: RwLock<Option<ProcessHandle>> = RwLock::new(None);

impl StdString {
    fn read(&self, handle: ProcessHandle) -> Result<String> {
        if self.len <= 15 {
            let data = &self.buf[..self.len as usize];
            return Ok(String::from_utf8(data.to_owned())?);
        }

        let ptr = u32::from_le_bytes(self.buf[..4].try_into().unwrap());
        let mut buf = vec![0; self.len as usize];

        handle.copy_address(ptr as usize, &mut buf)?;

        Ok(String::from_utf8(buf)?)
    }
}

impl Debug for StdString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        {
            let debug_handle = DEBUG_HANDLE.read().unwrap();
            if let Some(handle) = *debug_handle {
                return self.read(handle).unwrap().fmt(f);
            }
        }
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
    /// Use a custom address map read from a given file. When no argument is given prints the default one
    #[arg(long)]
    address_map: Option<Option<PathBuf>>,
}

fn read_address_map(path: &Path) -> Result<toml::Table> {
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}

fn main() -> Result<()> {
    let args = Args::parse();

    let address_map = include_str!("address-map.toml");
    let mut address_map = match args.address_map {
        Some(None) => {
            println!("{address_map}");
            return Ok(());
        }
        Some(Some(custom)) => read_address_map(&custom).context("Reading custom address map")?,
        None => toml::from_str(address_map)?,
    };

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
    *DEBUG_HANDLE.write().unwrap() = Some(handle);

    let u32s = match address_map.remove("u32") {
        Some(toml::Value::Table(u32s)) => u32s,
        Some(_) => bail!("Invalid address map: `u32` is not a table"),
        None => toml::Table::new(),
    };

    let mut data = HashMap::new();
    for (k, v) in u32s {
        let toml::Value::Integer(addr) = v else {
            bail!("Invalid address map: `u32.{k}` is not a number")
        };
        let ptr = RemotePtr::<u32>::new(addr as u32);

        data.insert(k, ptr.read(handle)?);
    }

    match address_map.get("stats-map") {
        Some(toml::Value::Integer(addr)) => {
            let map_ptr = RemotePtr::<RemotePtr<StringIntMapNode>>::new(*addr as u32);
            let map = map_ptr.read(handle)?;
            data.insert(
                "wins".to_owned(),
                map.get(handle, "progress_ending0")?.unwrap_or_default()
                    + map.get(handle, "progress_ending1")?.unwrap_or_default(),
            );
        }
        Some(_) => bail!("Invalid address map: `stats-map` is not a number"),
        None => {}
    }

    let formatted = args.format.format(&data).map_err(|e| match e {
        FmtError::Invalid(msg) | FmtError::KeyError(msg) | FmtError::TypeError(msg) => {
            anyhow!("Result format error: {msg}")
        }
    })?;

    println!("{formatted}");

    Ok(())
}
