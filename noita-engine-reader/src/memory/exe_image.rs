use std::{
    ffi::CStr,
    io,
    ops::{Deref, Range},
};

use iced_x86::{Code, Decoder, DecoderOptions, Instruction};
use memchr::memmem;
use thiserror::Error;

use crate::memory::ProcessRef;

use super::PtrReadable;

#[derive(Error, Debug)]
pub enum ReadImageError {
    #[error("No MZ header, not win32")]
    InvalidMzHeader,
    #[error("Invalid PE header")]
    InvalidPeHeader,
    #[error("Missing .{0} section")]
    NoSection(&'static str),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, PtrReadable)]
#[repr(C)]
struct DosHeaderData {
    magic: [u8; 2],
    _skip: [u8; 0x3a],
    e_lfanew: u32, // offset to PE header
}

#[derive(Debug, PtrReadable)]
#[repr(C)]
struct PeHeaderData {
    magic: [u8; 4],
    machine: u16,
    number_of_sections: u16,
    time_date_stamp: u32,
    pointer_to_symbol_table: u32,
    number_of_symbols: u32,
    size_of_optional_header: u16,
    characteristics: u16,

    // optional header
    _skip: [u8; 56],
    size_of_image: u32,
}

#[derive(Debug, PtrReadable)]
#[repr(C)]
struct PeSectionHeader {
    name: [u8; 8],
    virtual_size: u32,
    virtual_address: u32,
    _skip: [u8; 24],
}

impl PeSectionHeader {
    fn range(&self) -> Range<usize> {
        let start = self.virtual_address as usize;
        start..(start + self.virtual_size as usize)
    }
}

#[derive(Debug, Clone)]
pub struct PeSection<'i> {
    base: usize,
    range: Range<usize>,
    section: &'i [u8],
    name: &'static str,
}

impl<'i> PeSection<'i> {
    pub fn scan(&self, needle: &[u8]) -> Option<usize> {
        let found =
            memmem::find(self.section, needle).map(|pos| (self.base + self.range.start + pos));

        if let Some(res) = found {
            tracing::debug!("Found needle {needle:?} in .{} at 0x{res:x}", self.name);
        } else {
            tracing::warn!("Did not find needle {needle:?} in .{}", self.name);
        }

        found
    }
}

#[derive(Debug, Clone)]
pub struct PeHeader {
    timestamp: u32,
    text: Range<usize>,
    rdata: Range<usize>,
    data: Range<usize>,
    image_size: u32,
}

impl PeHeader {
    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    pub fn read(proc: &ProcessRef) -> Result<Self, ReadImageError> {
        let base = proc.base();
        let dos_header = proc.read::<DosHeaderData>(base as _)?;
        if dos_header.magic != *b"MZ" {
            return Err(ReadImageError::InvalidMzHeader);
        }

        let pe = proc.read::<PeHeaderData>(base as u32 + dos_header.e_lfanew)?;
        if pe.magic != *b"PE\0\0" {
            return Err(ReadImageError::InvalidPeHeader);
        }

        let sections = proc.read_multiple::<PeSectionHeader>(
            base as u32
                + dos_header.e_lfanew
                // + size_of::<PeHeaderData>() as u32
                + 24 // size of PeHeader without(!) optional header
                + pe.size_of_optional_header as u32,
            pe.number_of_sections as u32,
        )?;

        let text = sections
            .iter()
            .find(|s| &s.name == b".text\0\0\0")
            .ok_or(ReadImageError::NoSection("text"))?;

        let rdata = sections
            .iter()
            .find(|s| &s.name == b".rdata\0\0")
            .ok_or(ReadImageError::NoSection("rdata"))?;

        let data = sections
            .iter()
            .find(|s| &s.name == b".data\0\0\0")
            .ok_or(ReadImageError::NoSection("data"))?;

        Ok(Self {
            timestamp: pe.time_date_stamp,
            text: text.range(),
            rdata: rdata.range(),
            data: data.range(),
            image_size: pe.size_of_image,
        })
    }
}

#[derive(Debug)]
pub struct ExeImage {
    proc: ProcessRef,
    image: Vec<u8>,
}

impl Deref for ExeImage {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}

impl ExeImage {
    /// This is relatively slow, as we read the entire executable (according to
    /// it's image size from the PE header) from the process memory
    pub fn read(proc: &ProcessRef) -> Result<Self, io::Error> {
        Ok(Self {
            proc: proc.clone(),
            image: proc.read_multiple(proc.base() as _, proc.header().image_size)?,
        })
    }

    pub fn text(&self) -> PeSection<'_> {
        let range = self.proc.header().text.clone();
        PeSection {
            base: self.proc.base(),
            section: &self[range.clone()],
            range,
            name: "text",
        }
    }

    pub fn rdata(&self) -> PeSection<'_> {
        let range = self.proc.header().rdata.clone();
        PeSection {
            base: self.proc.base(),
            section: &self[range.clone()],
            range,
            name: "rdata",
        }
    }

    pub fn data(&self) -> PeSection<'_> {
        let range = self.proc.header().data.clone();
        PeSection {
            base: self.proc.base(),
            section: &self[range.clone()],
            range,
            name: "data",
        }
    }

    pub fn header(&self) -> &PeHeader {
        self.proc.header()
    }

    pub fn base(&self) -> usize {
        self.proc.base()
    }

    /// Find the address of a `PUSH <given string>` instruction
    pub fn find_push_str(&self, needle: &CStr) -> Option<usize> {
        let string = self.rdata().scan(needle.to_bytes_with_nul())?;
        let [a, b, c, d] = (string as u32).to_le_bytes();
        self.text().scan(&[0x68, a, b, c, d])
    }

    /// Not guaranteed to end at the current function, as we only check for a few return opcodes and int3
    pub fn decode_fn(&self, addr: u32) -> impl Iterator<Item = Instruction> + '_ {
        Decoder::with_ip(
            32,
            &self.image[addr as usize - self.proc.base()..],
            addr as u64,
            DecoderOptions::NONE,
        )
        .into_iter()
        .take_while(|instr| {
            instr.code() != Code::Int3
                && instr.code() != Code::Retnd
                && instr.code() != Code::Retnd_imm16
        })
    }

    pub fn find_vftable(&self, mangled_type_name: &CStr) -> Option<u32> {
        // first we find the part of the RTTI type descriptor that contains
        // the type name that should not ever change (I hope), and get the
        // descriptor address from that
        let descriptor = self.data().scan(mangled_type_name.to_bytes_with_nul())? as u32 - 8;

        // then we construct the *expected* RTTI locator prefix
        // (with signature, offset and cdOffset dwords being 0)
        let [a, b, c, d] = descriptor.to_le_bytes();
        let locator_bytes = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, a, b, c, d,
        ];

        // and find its address
        let locator = self.rdata().scan(&locator_bytes)? as u32;

        // which is pointed to from a place right before the vftable
        let vftable = self.rdata().scan(&locator.to_le_bytes())? as u32 + 4;

        tracing::debug!("Found vftable for {mangled_type_name:?} at {vftable:x}");

        Some(vftable)
    }

    pub fn find_static_global(&self, mangled_type_name: &CStr) -> Option<u32> {
        let vftable = self.find_vftable(mangled_type_name)?.to_le_bytes();
        let addr = self.data().scan(&vftable)?;
        tracing::debug!("Found static global for {mangled_type_name:?} at 0x{addr:x}",);
        Some(addr as _)
    }
}
