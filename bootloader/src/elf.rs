use core::{
    mem::MaybeUninit::{self},
    slice,
};

use alloc::vec::Vec;
use core::convert::TryInto;

pub struct Elf<'a> {
    pub abi: Abi,
    pub object_type: ObjectType,
    pub entry: u64,
    pub program_headers: Vec<HeaderEntry<'a>>,
    pub section_headers: Vec<SectionEntry<'a>>,
    pub flags: u32,
    pub section_name_section_index: usize,
}

fn slice_to_array<T, const SIZE: usize>(slice: &[T]) -> [T; SIZE] {
    assert_eq!(slice.len(), SIZE);
    let mut ret = MaybeUninit::<T>::uninit_array::<SIZE>();
    // Safety:
    // - src (slice) is valid for SIZE reads of T, checked with assert above
    // - dst (ret) is valid for SIZE writes of T, due to it being of size SIZE
    // - due to both src and dst being directly from slices of T (and MaybeUninit<T> having same layout as T),
    //   both are properly aligned
    // - src and dst do not overlap
    unsafe { core::ptr::copy_nonoverlapping(slice.as_ptr(), ret.as_mut_ptr() as _, SIZE) };
    // Safety:
    // - All elements are initialized per the above copy
    unsafe { MaybeUninit::array_assume_init(ret) }
}

impl<'a> Elf<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Elf<'a>, ()> {
        // Magic
        if data[0..4] != [0x7F, 0x45, 0x4C, 0x46] {
            return Err(());
        }

        // 32- or 64-bit
        if data[4] != 2 {
            return Err(());
        }
        // Endianness
        if data[5] != 1 {
            return Err(());
        }
        // Version
        if data[6] != 1 {
            return Err(());
        }

        let abi = if data[7] == 0 {
            Abi::SystemV
        } else {
            Abi::Other
        };
        let object_type = match u16::from_le_bytes(slice_to_array(&data[0x10..0x12])) {
            0 => ObjectType::None,
            1 => ObjectType::Rel,
            2 => ObjectType::Exec,
            3 => ObjectType::Dyn,
            4 => ObjectType::Core,
            0xFE00..=0xFFFF => ObjectType::Other,
            _ => return Err(()),
        };

        // Machine
        if &data[0x12..0x14] != &0x3Eu16.to_le_bytes() {
            return Err(());
        }
        // Version, again
        if &data[0x14..0x18] != &1u32.to_le_bytes() {
            return Err(());
        }

        let entry = u64::from_le_bytes(slice_to_array(&data[0x18..0x20]));
        let program_headers_offset = u64::from_le_bytes(slice_to_array(&data[0x20..0x28]));
        let section_headers_offset = u64::from_le_bytes(slice_to_array(&data[0x28..0x30]));
        let flags = u32::from_le_bytes(slice_to_array(&data[0x30..0x34]));

        // File header size
        if u16::from_le_bytes(slice_to_array(&data[0x34..0x36])) != 64 {
            return Err(());
        }

        let program_header_size = u16::from_le_bytes(slice_to_array(&data[0x36..0x38])) as u64;
        let program_header_count = u16::from_le_bytes(slice_to_array(&data[0x38..0x3A])) as u64;
        let section_header_size = u16::from_le_bytes(slice_to_array(&data[0x3A..0x3C])) as u64;
        let section_header_count = u16::from_le_bytes(slice_to_array(&data[0x3C..0x3E])) as u64;
        let section_name_section_index =
            u16::from_le_bytes(slice_to_array(&data[0x3E..0x40])) as usize;

        let mut program_headers = Vec::with_capacity(program_header_count as _);
        for i in 0..program_header_count {
            let offset = program_headers_offset + program_header_size * i;
            let offset = offset as usize;
            let data_offset =
                u64::from_le_bytes(data[offset + 0x8..offset + 0x10].try_into().unwrap());
            let entry_type = match u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
            {
                0 => EntryType::None,
                1 => {
                    println!("Loadable section with data offset {:x}", data_offset);
                    EntryType::Load
                }
                2 => EntryType::Dynamic,
                3 => EntryType::Interp,
                4 => EntryType::Note,
                5 => EntryType::Shlib,
                6 => EntryType::Phdr,
                7 => EntryType::Tls,
                0x60000000..=0x7FFFFFFF => EntryType::Other,
                _ => return Err(()),
            };
            let flags = u32::from_le_bytes(data[offset + 0x4..offset + 0x8].try_into().unwrap());
            let virtual_addr =
                u64::from_le_bytes(data[offset + 0x10..offset + 0x18].try_into().unwrap());
            let physical_addr =
                u64::from_le_bytes(data[offset + 0x18..offset + 0x20].try_into().unwrap());
            let file_size =
                u64::from_le_bytes(data[offset + 0x20..offset + 0x28].try_into().unwrap());
            let mem_size =
                u64::from_le_bytes(data[offset + 0x28..offset + 0x30].try_into().unwrap());
            let align = u64::from_le_bytes(data[offset + 0x30..offset + 0x38].try_into().unwrap());

            program_headers.push(HeaderEntry {
                entry_type,
                flags,
                data: &data[data_offset as usize..(data_offset + file_size) as usize],
                virtual_addr,
                physical_addr,
                mem_size,
                align,
            });
        }

        let mut section_headers = Vec::with_capacity(section_header_count as _);
        for i in 0..section_header_count {
            let offset = section_headers_offset + section_header_size * i;
            let offset = offset as usize;
            let section_type =
                match u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()) {
                    0 => SectionType::None,
                    1 => SectionType::Progbits,
                    2 => SectionType::Symtab,
                    3 => SectionType::Strtab,
                    4 => SectionType::Rela,
                    5 => SectionType::Hash,
                    6 => SectionType::Dynamic,
                    7 => SectionType::Note,
                    8 => SectionType::Nobits,
                    9 => SectionType::Rel,
                    11 => SectionType::Dynsym,
                    14 => SectionType::InitArray,
                    15 => SectionType::FiniArray,
                    16 => SectionType::PreinitArray,
                    17 => SectionType::Group,
                    18 => SectionType::SymtabShndx,
                    19 => SectionType::Num,
                    o @ 0x60000000..=0xFFFFFFFF => SectionType::Other(o),
                    _ => return Err(()),
                };
            let flags = u64::from_le_bytes(data[offset + 0x8..offset + 0x10].try_into().unwrap());
            let virtual_addr =
                u64::from_le_bytes(data[offset + 0x10..offset + 0x18].try_into().unwrap());
            let data_offset =
                u64::from_le_bytes(data[offset + 0x18..offset + 0x20].try_into().unwrap());
            let file_size =
                u64::from_le_bytes(data[offset + 0x20..offset + 0x28].try_into().unwrap());
            let link = u32::from_le_bytes(data[offset + 0x28..offset + 0x2C].try_into().unwrap());
            let info = u32::from_le_bytes(data[offset + 0x2C..offset + 0x30].try_into().unwrap());
            let align = u64::from_le_bytes(data[offset + 0x30..offset + 0x38].try_into().unwrap());
            let entry_size =
                u64::from_le_bytes(data[offset + 0x38..offset + 0x40].try_into().unwrap());

            section_headers.push(SectionEntry {
                name: "",
                section_type,
                flags,
                data: &data[data_offset as usize..(data_offset + file_size) as usize],
                virtual_addr,
                link,
                info,
                align,
                entry_size,
            });
        }

        Ok(Elf {
            abi,
            object_type,
            entry,
            program_headers,
            section_headers,
            flags,
            section_name_section_index,
        })
    }
}

pub struct HeaderEntry<'a> {
    pub entry_type: EntryType,
    pub flags: u32,
    pub data: &'a [u8],
    pub virtual_addr: u64,
    pub physical_addr: u64,
    pub mem_size: u64,
    pub align: u64,
}

pub struct SectionEntry<'a> {
    pub name: &'a str,
    pub section_type: SectionType,
    pub flags: u64,
    pub data: &'a [u8],
    pub virtual_addr: u64,
    pub link: u32,
    pub info: u32,
    pub align: u64,
    pub entry_size: u64,
}

pub enum SectionType {
    None,
    Progbits,
    Symtab,
    Strtab,
    Rela,
    Hash,
    Dynamic,
    Note,
    Nobits,
    Rel,
    Dynsym,
    InitArray,
    FiniArray,
    PreinitArray,
    Group,
    SymtabShndx,
    Num,
    Other(u32),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    None,
    Load,
    Dynamic,
    Interp,
    Note,
    Shlib,
    Phdr,
    Tls,
    Other,
}

pub enum Abi {
    SystemV,
    Other,
}

pub enum ObjectType {
    None,
    Rel,
    Exec,
    Dyn,
    Core,
    Other,
}
