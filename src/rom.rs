use std::num::Wrapping;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use crc::{Crc, CRC_32_ISO_HDLC};
use crate::elf::Elf;

/// Used to determine IPL3 variant
pub const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

/// Represents an N64 ROM header with all known header fields.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Header {
    /// The first 4 bytes of the header are used by IPL2 to initialize the PI DOM1_xxx registers.
    /// Emulators often use them to determine the endianness of the ROM, but they can be different
    /// values than the standard found in all official game releases.
    pub pi_regs: u32,
    pub clockrate: u32,
    /// Also known as the entrypoint, however different IPL3 variants treat this value differently
    /// (e.g. some will offset it by some amount first.)
    pub pc: u32,
    pub unknown0: u16,
    pub release: u16,
    pub checksum: u64,
    pub unknown1: u64,
    pub image_name: [u8; 20],
    pub unknown2: [u8; 7],
    pub media_format: u8,
    pub cart_id: u16,
    pub country: u8,
    pub revision: u8,
}
impl Header {
    /// Parses binary header data into a [`Header`]. 
    pub fn new(data: [u8; 0x40]) -> Self {
        let mut data = Bytes::from(data.to_vec());
        
        Self {
            pi_regs: data.get_u32(),
            clockrate: data.get_u32(),
            pc: data.get_u32(),
            unknown0: data.get_u16(),
            release: data.get_u16(),
            checksum: data.get_u64(),
            unknown1: data.get_u64(),
            image_name: { data.advance(20); data.slice(0x20..0x34)[..].try_into().unwrap() },
            unknown2: { data.advance(7); data.slice(0x34..0x3B)[..].try_into().unwrap() },
            media_format: data.get_u8(),
            cart_id: data.get_u16(),
            country: data.get_u8(),
            revision: data.get_u8(),
        }
    }
    
    /// Generates a new [`Header`] using the binary part of a rom, an IPL3, name, and entrypoint.
    /// 
    /// Use [`Self::new()`] to parse existing header data.
    pub fn generate(binary: &[u8], ipl3: [u8; 0x1000 - 0x40], name: &str, entry: u32) -> Self {
        let mut combined = BytesMut::with_capacity(binary.len() + ipl3.len());
        combined.extend_from_slice(binary);
        combined.extend_from_slice(&ipl3);
        
        let name: [u8; 20] = format!("{:20}", name).as_bytes().try_into().unwrap();
        
        let checksum = Self::calculate_checksum(binary, ipl3);
        
        Self {
            pi_regs: 0x80371240,
            clockrate: 0x0000000F,
            pc: entry,
            unknown0: 0x0000,
            release: 0x1E4E, // who needs libultra when you have rust?
            checksum,
            unknown1: 0x0000000000000000,
            image_name: name,
            unknown2: [0x00; 7],
            media_format: 0x52, // "R" (rust)
            cart_id: 0x3634, // "64"
            country: 0x37, // "7" (beta)
            revision: 0x01
        }
    }
    
    /// Encodes the header data into a `Vec`.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut data = BytesMut::with_capacity(0x40);
        
        data.put_u32(self.pi_regs);
        data.put_u32(self.clockrate);
        data.put_u32(self.pc);
        data.put_u16(self.unknown0);
        data.put_u16(self.release);
        data.put_u64(self.checksum);
        data.put_u64(self.unknown1);
        data.put_slice(&self.image_name);
        data.put_slice(&self.unknown2);
        data.put_u8(self.media_format);
        data.put_u16(self.cart_id);
        data.put_u8(self.country);
        data.put_u8(self.revision);
        
        data.to_vec()
    }
    
    /// Computes the 64-bit checksum found in N64 ROM headers.
    /// 
    /// This algorithm is practically nonsense and was likely designed for "security through
    /// obscurity", like many checksum algorithms developed by Nintendo at the time.
    /// 
    /// The checksum depends on the IPL3 being used. Custom IPL3s will cause this function
    /// to return a checksum of `0u64`. This may be changed in future versions.
    /// 
    /// Original source: http://n64dev.org/n64crc.html
    pub fn calculate_checksum(binary: &[u8], ipl3: [u8; 0x1000 - 0x40]) -> u64 {
        #[derive(PartialEq)]
        enum Variant {
            X103,
            X105,
            X106,
            Others,
        }
        use Variant::*;
        
        // The initial value is decided based on which IPL3 variant is used
        // initial = (seed * magic_number) + 1
        //
        // The seed is hardcoded into each CIC variant, and the magic number is hardcoded into the
        // matching IPL3 variant. However, even though 6101, 6102/7101, and 7102 are three different
        // variants, they use the same seed and magic number.
        let (initial, variant) = match CRC.checksum(&ipl3) {
            0x6170A4A1 | 0x90BB6CB5 | 0x009E9EA3 => (((0x3Fu64 * 0x5D588B65u64) + 1) as u32, Others), // 6101, 6102/7101, 7102
            0x0B050EE0 => (((0x78u64 * 0x6C078965u64) + 1) as u32, X103), // 6103/7103
            0x98BC2C86 => (((0x91u64 * 0x5D588B65u64) + 1) as u32, X105), // 6105/7105
            0xACC8580A => (((0x85u64 * 0x6C078965u64) + 1) as u32, X106), // 6106/7106
            _ => return 0
        };
        
        let mut t1 = Wrapping(initial);
        let mut t2 = Wrapping(initial);
        let mut t3 = Wrapping(initial);
        let mut t4 = Wrapping(initial);
        let mut t5 = Wrapping(initial);
        let mut t6 = Wrapping(initial);
        
        let mut data = Bytes::from(binary[..0x100000].to_vec());
        let mut table = Bytes::from(ipl3[0x710..0x750].to_vec());
        
        while data.has_remaining() {
            let word = data.get_u32();
            
            let rot = Wrapping(word.rotate_left(word & 0x1F));
            let word = Wrapping(word);
            
            if (t6 + word) < t6 {
                t4 += 1;
            }
            
            t6 += word;
            t3 ^= word;
            t5 += rot;
            
            if t2 > word {
                t2 ^= rot;
            } else {
                t2 ^= t6 ^ word;
            }
            
            if variant == X105 {
                t1 += Wrapping(table.get_u32()) ^ word;
                if !table.has_remaining() {
                    table = Bytes::from(table.to_vec());
                }
            } else {
                t1 += t5 ^ word;
            }
        }
        
        match variant {
            X103 => ((((t6 ^ t4) + t3).0 as u64) << 32) | (((t5 ^ t2) + t1).0 as u64),
            X106 => ((((t6 * t4) + t3).0 as u64) << 32) | (((t5 * t2) + t1).0 as u64),
            _ =>    ((((t6 ^ t4) ^ t3).0 as u64) << 32) | (((t5 ^ t2) ^ t1).0 as u64)
        }
    }
}

/// Represents an N64 ROM binary split into the parts: the header, IPL3, and remaining binary.
#[derive(Clone, Debug, PartialEq)]
pub struct Rom {
    pub header: Header,
    /// Initial Program Load Stage 3, run during the boot process of the console.
    pub ipl3: [u8; 0x1000 - 0x40],
    /// The remaining binary code found after the IPL3 section.
    pub binary: Vec<u8>,
}
impl Rom {
    /// Extracts necessary data from an [`Elf`] to generate an N64-compatible ROM.
    /// 
    /// The ROM header will be auto-generated based on the Elf. If `name` is Some, it will be used
    /// in the ROM's header. Otherwise the name of the Elf artifact will be used.
    /// 
    /// In either case, the name will be trimmed or padded with ASCII spaces to exactly 20 bytes. 
    pub fn new(elf: &Elf, ipl3: [u8; 0x1000 - 0x40], name: Option<&str>) -> Self {
        let mut binary = BytesMut::new();
        let boot = elf.sections.get(".boot").expect(".boot section missing");
        binary.put_slice(&boot.data);
        
        let mut offset = boot.addr + boot.data.len() as u64;
        
        for name in [".text", ".rodata", ".data", ".got"] {
            if let Some(section) = elf.sections.get(name) {
                if section.data.len() == 0 { continue; }
                
                let section_addr = section.addr;
                if offset < section_addr { // if needed, pad binary until the next section starts
                    binary.resize(binary.len() + (section_addr - offset) as usize, 0x00);
                    offset = section_addr;
                }
                
                binary.extend_from_slice(&section.data);
                
                offset += section.data.len() as u64;
            }
        }
        
        // if binary smaller than 1MB, pad to 1MB
        if binary.len() < 0x100000 {
            binary.resize(0x100000, 0x00);
        }
        
        Self {
            header: Header::generate(&binary, ipl3, name.unwrap_or(&elf.path.file_name().unwrap().to_string_lossy()), elf.entry),
            ipl3,
            binary: binary.to_vec(),
        }
    }
    
    /// Copies ROM components into a Vec.
    /// 
    /// Use this to combine `self`'s header, IPL3, and remaining code/assets into a usable N64 ROM.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut data = BytesMut::with_capacity(0x40 + self.ipl3.len() + self.binary.len());
        
        data.put_slice(&self.header.to_vec());
        data.put_slice(&self.ipl3);
        data.put_slice(&self.binary);
        
        data.to_vec()
    }
}