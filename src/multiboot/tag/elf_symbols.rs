use bitflags::bitflags;
use core::fmt::{Debug, Formatter, Result};
use core::marker::PhantomData;
use core::mem::size_of;

use super::InternalCStr;
use super::TagHeader;
use crate::memory::MemoryRegion;

const ELF32_SHDR_SIZE: u32 = size_of::<Elf32Shdr>() as u32;
const ELF64_SHDR_SIZE: u32 = size_of::<Elf64Shdr>() as u32;

pub struct ElfSymbols {
    header: TagHeader,
    num: u32,
    entry_size: u32,
    shndx: u32,
    // section headers start after here
}

impl ElfSymbols {
    pub fn sections(&self) -> Iter {
        let first_entry = self.section_list_start();
        Iter {
            current_entry: first_entry,
            entry_size: self.entry_size,
            entries_remaining: self.num,
            string_section: self.string_section(),
            _marker: PhantomData,
        }
    }

    /// Returns a MemoryRegion that bounds where the kernel resides in memory
    pub fn kernel_memory_region(&self) -> MemoryRegion {
        let start = self
            .sections()
            .filter_map(|section| {
                if section.section_type() != SectionType::Null {
                    Some(section.addr())
                } else {
                    None
                }
            })
            .min()
            .unwrap();
        let end = self
            .sections()
            .filter_map(|section| {
                if section.section_type() != SectionType::Null {
                    Some(section.addr())
                } else {
                    None
                }
            })
            .max()
            .unwrap();
        MemoryRegion::new(start as usize, end as usize)
    }

    fn section_list_start(&self) -> *const u8 {
        // SAFETY: This is safe because `self.offset(1)` will return the first byte past the
        //         ElfSymbols struct in memory, the computed offset cannot overflow an isize, and
        //         the offset will not wrap around the address space (because the structs are populated
        //         by the multiboot2 header to fit in physical memory) TODO: are we using vaddrs or
        //         paddrs? (see ptr.offset())
        unsafe { (self as *const ElfSymbols).offset(1) as *const u8 }
    }

    fn string_section(&self) -> StringSection {
        let string_section_addr = self.section_list_start() as u32 + self.shndx * self.entry_size;

        // SAFETY: We know the address is properly aligned, as the entry size is a multiple of 8.
        //         We also know it points to either a valid 64-bit Shdr or 32-bit Shdr due to the
        //         multiboot2 specification.
        let shdr: &dyn ElfShdr = unsafe {
            match self.entry_size {
                ELF32_SHDR_SIZE => &*(string_section_addr as *const Elf32Shdr),
                ELF64_SHDR_SIZE => &*(string_section_addr as *const Elf64Shdr),
                _ => panic!("Unknown Elf Shdr size!"),
            }
        };

        StringSection { shdr }
    }
}

pub struct Iter<'a> {
    current_entry: *const u8,
    entry_size: u32,
    entries_remaining: u32,
    string_section: StringSection<'a>,
    _marker: PhantomData<&'a dyn ElfShdr>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = ElfSection<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.entries_remaining == 0 {
            return None;
        }

        // SAFETY: We know the entry is properly aligned, as we are always incrementing the
        //         `current_entry` pointer by 40 or 64 bytes (which is a multiple of 8). Similarly,
        //         we know it points to a valid 64-bit Shdr or 32-bit Shdr because we know how many
        //         entries remain due to the information in the multiboot2 tag, and the list always
        //         contains valid entries of those types.
        let shdr: &dyn ElfShdr = unsafe {
            match self.entry_size {
                ELF32_SHDR_SIZE => &*(self.current_entry as *const Elf32Shdr),
                ELF64_SHDR_SIZE => &*(self.current_entry as *const Elf64Shdr),
                _ => panic!("Unknown Elf Shdr size!"),
            }
        };

        self.current_entry = (self.current_entry as usize + self.entry_size as usize) as *const u8;
        self.entries_remaining -= 1;

        Some(ElfSection {
            shdr,
            string_section: self.string_section,
        })
    }
}

macro_rules! delegate_to_inner {
    ($func:ident, $ret_type:ty) => {
        fn $func(&self) -> $ret_type {
            self.shdr.$func()
        }
    };
}

#[derive(Debug, Eq, PartialEq)]
pub enum SectionType {
    Null,
    ProgramBits,
    SymbolTable,
    StringTable,
    Rela,
    SymbolHashTable,
    DynamicLinkingTable,
    Note,
    NoBits,
    Rel,
    Reserved,
    DynamicLoaderSymbolTable,
    EnvironmentSpecific,
    ProcessorSpecific,
    Unknown,
}

bitflags! {
    pub struct SectionFlags: u64 {
        const WRITE = 0x1;
        const ALLOC = 0x2;
        const EXEC = 0x4;
        // how is maskos and maskproc used?
    }
}

pub struct ElfSection<'a> {
    shdr: &'a dyn ElfShdr,
    string_section: StringSection<'a>,
}

impl<'a> ElfSection<'a> {
    pub fn name(&self) -> &str {
        // SAFETY: We know that this a safe lookup, because the string_section is populated from
        //         the multiboot2 header in the ElfSymbols struct. We know that it gets the
        //         correct entry, because it correctly calculates the entry based on the shndx.
        unsafe { self.string_section.lookup(self.name_offset() as isize) }
    }

    pub fn section_type(&self) -> SectionType {
        match self.shdr.section_type() {
            0 => SectionType::Null,
            1 => SectionType::ProgramBits,
            2 => SectionType::SymbolTable,
            3 => SectionType::StringTable,
            4 => SectionType::Rela,
            5 => SectionType::SymbolHashTable,
            6 => SectionType::DynamicLinkingTable,
            7 => SectionType::Note,
            8 => SectionType::NoBits,
            9 => SectionType::Rel,
            10 => SectionType::Reserved,
            11 => SectionType::DynamicLoaderSymbolTable,
            0x6000_0000..=0x6FFF_FFFF => SectionType::EnvironmentSpecific,
            0x7000_0000..=0x7FFF_FFFF => SectionType::ProcessorSpecific,
            _ => SectionType::Unknown,
        }
    }

    pub fn section_type_raw(&self) -> u32 {
        self.shdr.section_type()
    }

    pub fn flags(&self) -> SectionFlags {
        SectionFlags::from_bits_truncate(self.shdr.flags())
    }

    delegate_to_inner!(name_offset, u32);
    delegate_to_inner!(addr, u64);
    delegate_to_inner!(offset, u64);
    delegate_to_inner!(size, u64);
    delegate_to_inner!(link, u32);
    delegate_to_inner!(info, u32);
    delegate_to_inner!(addr_alignment, u64);
    delegate_to_inner!(entry_size, u64);
}

impl<'a> Debug for ElfSection<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ElfSection")
            .field("name", &self.name())
            .field("section_type", &self.section_type())
            .field("flags", &self.flags())
            .field("addr", &self.addr())
            .field("offset", &self.offset())
            .field("size", &self.size())
            .finish()
    }
}

macro_rules! impl_elf_section {
    ($struct:ident) => {
        impl ElfShdr for $struct {
            fn name_offset(&self) -> u32 {
                self.name_offset as u32
            }

            fn section_type(&self) -> u32 {
                self.section_type as u32
            }

            fn flags(&self) -> u64 {
                self.flags as u64
            }

            fn addr(&self) -> u64 {
                self.addr as u64
            }

            fn offset(&self) -> u64 {
                self.offset as u64
            }

            fn size(&self) -> u64 {
                self.size as u64
            }

            fn link(&self) -> u32 {
                self.link as u32
            }

            fn info(&self) -> u32 {
                self.info as u32
            }

            fn addr_alignment(&self) -> u64 {
                self.addr_alignment as u64
            }

            fn entry_size(&self) -> u64 {
                self.entry_size as u64
            }
        }
    };
}

/// Allows us to be generic between Elf32 and Elf64 sections
trait ElfShdr: core::fmt::Debug {
    fn name_offset(&self) -> u32;
    // TODO: use enums here
    fn section_type(&self) -> u32;
    fn flags(&self) -> u64;
    fn addr(&self) -> u64;
    fn offset(&self) -> u64;
    fn size(&self) -> u64;
    fn link(&self) -> u32;
    fn info(&self) -> u32;
    fn addr_alignment(&self) -> u64;
    fn entry_size(&self) -> u64;
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct Elf32Shdr {
    /// The offset (in bytes) in the section string table to find the section name string.
    name_offset: u32,

    /// The type of the section
    section_type: u32,

    /// Attribute flags of the section (i.e. writable, executable, etc).
    flags: u32,

    /// The virtual address of the beggining of the section in memory
    addr: u32,

    /// The offset in bytes of the beginning of the section contents in the file
    offset: u32,

    /// The size (in bytes) of the section
    size: u32,

    /// The section index of an associated section. Used for a few purposes (see Elf docs)
    link: u32,

    /// Extra information about the section (see Elf docs)
    info: u32,

    /// The required alignment of a section (must be power of 2)
    addr_alignment: u32,

    /// If the section contains fixed-size entries, this is the size of each entry in bytes.
    /// Otherwise, it is zero.
    entry_size: u32,
}

impl_elf_section!(Elf32Shdr);

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct Elf64Shdr {
    name_offset: u32,
    section_type: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addr_alignment: u64,
    entry_size: u64,
}

impl_elf_section!(Elf64Shdr);

/// Represents an Elf String section
#[derive(Debug, Copy, Clone)]
struct StringSection<'a> {
    shdr: &'a dyn ElfShdr,
}

impl<'a> StringSection<'a> {
    /// # Safety:
    /// This function is safe so long as `shdr` points to a valid string section. This should be
    /// the case as long we actually read it from the multiboot2 header and we did the pointer
    /// arithmetic correctly
    unsafe fn lookup(&self, offset: isize) -> &str {
        let table_ptr = self.shdr.addr() as *const u8;
        let str_ptr = table_ptr.offset(offset);
        let raw_str = &*(str_ptr as *const InternalCStr);
        raw_str.to_str()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::random;

    fn sections_equal(section: ElfSection, shdr: &dyn ElfShdr) -> bool {
        section.name_offset() == shdr.name_offset()
            && section.section_type_raw() == shdr.section_type()
            && section.flags().bits == shdr.flags()
            && section.addr() == shdr.addr()
            && section.offset() == shdr.offset()
            && section.size() == shdr.size()
            && section.link() == shdr.link()
            && section.info() == shdr.info()
            && section.addr_alignment() == shdr.addr_alignment()
            && section.entry_size() == shdr.entry_size()
    }

    fn random_elf64_shdr() -> Elf64Shdr {
        Elf64Shdr {
            name_offset: random(),
            section_type: random::<u32>() % 12,
            flags: random::<u64>() % 8,
            addr: random(),
            offset: random(),
            size: random(),
            link: random(),
            info: random(),
            addr_alignment: random(),
            entry_size: random(),
        }
    }

    fn random_elf32_shdr() -> Elf32Shdr {
        Elf32Shdr {
            name_offset: random(),
            section_type: random::<u32>() % 12,
            flags: random::<u32>() % 8,
            addr: random(),
            offset: random(),
            size: random(),
            link: random(),
            info: random(),
            addr_alignment: random(),
            entry_size: random(),
        }
    }

    #[test]
    fn iterate_elf64_shdr() {
        // create a random list of shdrs
        let shdrs = [random_elf64_shdr(); 5];
        // just a random string section NOTE: Don't call name() or lookup() -> this is super unsafe
        let string_shdr = random_elf64_shdr();
        let string_section = StringSection { shdr: &string_shdr };

        let iterator = Iter {
            current_entry: shdrs.as_ptr() as *const u8,
            entry_size: size_of::<Elf64Shdr>() as u32,
            entries_remaining: 5,
            string_section,
            _marker: PhantomData,
        };

        for (i, section) in iterator.enumerate() {
            assert!(sections_equal(section, &shdrs[i]));
        }
    }

    #[test]
    fn iterate_elf32_shdr() {
        // create a random list of shdrs
        let shdrs = [random_elf32_shdr(); 5];
        // just a random string section NOTE: Don't call name() or lookup() -> this will
        let string_shdr = random_elf32_shdr();
        let string_section = StringSection { shdr: &string_shdr };

        let iterator = Iter {
            current_entry: shdrs.as_ptr() as *const u8,
            entry_size: size_of::<Elf32Shdr>() as u32,
            entries_remaining: 5,
            string_section,
            _marker: PhantomData,
        };

        for (i, section) in iterator.enumerate() {
            assert!(sections_equal(section, &shdrs[i]));
        }
    }
}
