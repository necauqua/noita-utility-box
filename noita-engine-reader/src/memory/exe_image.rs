use std::{ffi::CStr, io, ops::Range};

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
    #[error("Unexpected PE Optional Header size: {0}")]
    UnexpectedOptionalHeaderSize(u16),
    #[error("Bad .text range in header {0:?}")]
    BadCodeRange(Range<usize>),
    #[error("Bad .rdata range in header {0:?}")]
    BadDataRange(Range<usize>),
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
    magic_and_linker_version: u32,
    size_of_code: u32,
    size_of_initialized_data: u32,
    size_of_uninitialized_data: u32,
    address_of_entry_point: u32,
    base_of_code: u32,
    base_of_data: u32,
    image_base: u32,
    _skip: [u8; 0x18],
    size_of_image: u32,
}

#[derive(Debug, Clone)]
pub struct PeHeader {
    timestamp: u32,
    text: Range<usize>,
    rdata: Range<usize>,
    image_base: u32,
    size_of_image: u32,
}

impl PeHeader {
    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    pub fn read(proc: &ProcessRef) -> Result<Self, ReadImageError> {
        let dos_header = proc.read::<DosHeaderData>(proc.base() as _)?;
        if dos_header.magic != *b"MZ" {
            return Err(ReadImageError::InvalidMzHeader);
        }

        let pe = proc.read::<PeHeaderData>(proc.base() as u32 + dos_header.e_lfanew)?;
        if pe.magic != *b"PE\0\0" {
            return Err(ReadImageError::InvalidPeHeader);
        }

        if pe.size_of_optional_header != 0xe0 {
            return Err(ReadImageError::UnexpectedOptionalHeaderSize(
                pe.size_of_optional_header,
            ));
        }

        let base_of_code = pe.base_of_code as usize;
        let size_of_code = pe.size_of_code as usize;
        let text = base_of_code..base_of_code + size_of_code;

        let base_of_data = pe.base_of_data as usize;
        let size_of_data = pe.size_of_initialized_data as usize;
        let rdata = base_of_data..base_of_data + size_of_data;

        let size_of_image = pe.size_of_image as usize;

        if text.start > size_of_image || text.end > size_of_image {
            return Err(ReadImageError::BadCodeRange(text));
        }
        if rdata.start > size_of_image || rdata.end > size_of_image {
            return Err(ReadImageError::BadDataRange(rdata));
        }

        Ok(Self {
            timestamp: pe.time_date_stamp,
            text,
            rdata,
            image_base: pe.image_base,
            size_of_image: pe.size_of_image,
        })
    }
}

#[derive(Debug)]
pub struct ExeImage {
    proc: ProcessRef,
    image: Vec<u8>,
    // cached_strings: HashMap<Vec<u8>, u32>,
    // cached_string_pushes: HashMap<Vec<u8>, usize>,
}

impl ExeImage {
    /// This is relatively slow, as we read the entire executable (according to
    /// it's image size from the PE header) from the process memory
    pub fn read(proc: &ProcessRef) -> Result<Self, io::Error> {
        let image = proc.read_multiple(proc.header().image_base, proc.header().size_of_image)?;

        let image = Self {
            proc: proc.clone(),
            image,
            // cached_strings: HashMap::new(),
            // cached_string_pushes: HashMap::new(),
        };

        //
        // The Aho-Corasick setup and search here takes a whopping ~1 second on
        // my machine; maybe I'm doing something horribly wrong (like having
        // two of them), but for now brute force wins
        //
        // Oh and the commented out code also doesn't work, probably some errors
        // in offsets that I never bothered to fix cuz it's so slow anyway
        //
        // Limiting the searches to .rdata/.text helped immensely though
        //

        // let mut cached_strings = HashMap::new();
        // let mut cached_string_pushes = HashMap::new();

        // let strings = [
        //     b"SetRandomSeed\0".as_ref(),
        //     b"GamePrint\0",
        //     b"AddFlagPersistent\0",
        //     b"EntityGetParent\0",
        //     b"EntityHasTag\0",
        //     b"EntityGetComponent\0",
        //     b"progress_ending1\0",
        //     b"Noita - Build ",
        // ];
        // let aho_corasick = aho_corasick::AhoCorasick::new(strings).unwrap();

        // let mut pushes = Vec::new();

        // for m in aho_corasick.find_iter(image.rdata()) {
        //     let addr = image.header.image_base + image.header.rdata.start as u32 + m.start() as u32;
        //     cached_strings.insert(strings[m.pattern()].to_vec(), addr);

        //     // skip last two lul
        //     if m.pattern().as_u32() <= 5 {
        //         let [a, b, c, d] = addr.to_le_bytes();
        //         pushes.push([0x68, a, b, c, d]);
        //     }
        // }
        // let aho_corasick = aho_corasick::AhoCorasick::new(&pushes).unwrap();
        // for m in aho_corasick.find_iter(image.text()) {
        //     cached_string_pushes.insert(strings[m.pattern()].to_vec(), m.start());
        // }

        // image.cached_strings = cached_strings;
        // image.cached_string_pushes = cached_string_pushes;

        Ok(image)
    }

    pub fn text(&self) -> &[u8] {
        &self.image[self.proc.header().text.clone()]
    }

    pub fn rdata(&self) -> &[u8] {
        &self.image[self.proc.header().rdata.clone()]
    }

    pub fn header(&self) -> &PeHeader {
        self.proc.header()
    }

    /// Find the program address of the given C string in rdata
    pub fn find_string(&self, needle: &CStr) -> Option<u32> {
        // if let Some(&res) = self.cached_strings.get(needle.to_bytes_with_nul()) {
        //     tracing::debug!("Found string {needle:?} at 0x{res:x}");
        //     return Some(res);
        // }

        let res = memmem::find(self.rdata(), needle.to_bytes_with_nul()).map(|pos| {
            (pos + self.proc.header().rdata.start + self.proc.header().image_base as usize) as u32
        });
        if let Some(res) = res {
            tracing::debug!("Found string {needle:?} at 0x{res:x}");
        } else {
            tracing::warn!("Did not find string {needle:?}");
        }
        res
    }

    /// Returns position *relative to .text*, not to the image base
    pub fn find_push_str_pos(&self, needle: &CStr) -> Option<usize> {
        // if let Some(&res) = self.cached_string_pushes.get(needle.to_bytes_with_nul()) {
        //     tracing::debug!("Found PUSH {needle:?} at offset 0x{res:x}",);
        //     return Some(res);
        // }

        let [a, b, c, d] = self.find_string(needle)?.to_le_bytes();
        let res = memmem::find(self.text(), &[0x68, a, b, c, d]);

        if let Some(res) = res {
            tracing::debug!("Found PUSH {needle:?} at offset 0x{res:x}",);
        } else {
            tracing::warn!("Did not find PUSH {needle:?}");
        }
        res
    }

    pub fn text_offset_to_addr(&self, offset: usize) -> u32 {
        (offset + self.proc.header().text.start) as u32 + self.proc.header().image_base
    }

    /// Not guaranteed to end at the current function, as we only check for a few return opcodes and int3
    pub fn decode_fn(&self, addr: u32) -> impl Iterator<Item = Instruction> + '_ {
        Decoder::with_ip(
            32,
            &self.text()[addr as usize
                - self.proc.header().image_base as usize
                - self.proc.header().text.start..],
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
}
