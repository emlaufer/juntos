pub mod elf_symbols;
pub mod memory_map;

use core::marker::PhantomData;
use core::{slice, str};

pub use elf_symbols::ElfSymbols;
pub use memory_map::MemoryMap;

pub struct TagIterator<'a> {
    current_tag: *const TagHeader,
    _marker: PhantomData<&'a TagHeader>,
}

impl<'a> TagIterator<'a> {
    /// # Safety
    /// This method is safe so long as `first_tag` correctly points to the first tag in a valid
    /// multiboot2 tag list, is non-null, and is properly aligned to an 8 byte boundary.
    pub unsafe fn new(first_tag: *const TagHeader) -> TagIterator<'a> {
        TagIterator {
            current_tag: first_tag,
            _marker: PhantomData,
        }
    }
}

impl<'a> Iterator for TagIterator<'a> {
    type Item = &'a TagHeader;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: We know current_tag is a valid pointer to a tag because the first is correct due
        //         to the safety of the constructor of the TagIterator, and the following tags are
        //         correct as multiboot2 ensures size (with alignment) will point to the next tag.
        let tag = unsafe { &*self.current_tag };

        // if ending tag, return None
        if tag.tag_type == 0 && tag.size == 8 {
            return None;
        }

        let next_tag_start = self.current_tag as usize + tag.size as usize;
        let next_tag_addr = (next_tag_start - 1 + 8) & !(8 - 1); // align to 8 byte boundary
        self.current_tag = next_tag_addr as *const TagHeader;

        Some(tag)
    }
}

/// A struct representing an 'internal' C-style string (i.e. within the struct itself)
#[repr(transparent)]
pub struct InternalCStr(u8);

impl InternalCStr {
    /// # Safety:
    /// This method is safe so long as `self` is a byte within a null-terminated UTF-8 string.
    /// NOTE: Be careful not to construct InternalCStr i.e. `let str = InternalCStr(*mem);`, as
    ///       this will copy the first byte. Instead, use `let str = &*(mem as *const
    ///       InternalCStr);` or as the last member of a struct.
    unsafe fn to_str(&self) -> &str {
        // SAFETY: The  string is a null-terminated UTF-8 string. We know the length
        //         is correct as we calculated it above by finding the null terminator.
        str::from_utf8_unchecked(slice::from_raw_parts(self.to_ptr(), self.len()))
    }

    fn to_ptr(&self) -> *const u8 {
        &self.0 as *const u8
    }

    /// # Safety:
    /// This method is safe so long as `self` is a byte within a null-terminated string.
    unsafe fn len(&self) -> usize {
        let mut len = 0;
        while *self.to_ptr().offset(len) != b'\0' {
            len += 1;
        }
        len as usize
    }
}

#[derive(Eq, PartialEq)]
pub enum Type {
    MemoryInfo,
    BiosBoot,
    BootCmdLine,
    Modules,
    ElfSymbols,
    MemoryMap,
    BootLoaderName,
    ApmTable,
    VbeInfo,
    FramebufferInfo,
    Efi32SystemTable,
    Efi64SystemTable,
    SMBiosTable,
    AcpiOldRdsp,
    AcpiNewRdsp,
    NetworkingInfo,
    EfiMemoryMap,
    EfiBootServicesNotTerminated,
    Efi32ImageHandle,
    Efi64ImageHandle,
    ImageLoadBase,
    Unknown,
}

#[derive(Debug)]
#[repr(C)]
pub struct TagHeader {
    pub tag_type: u32,
    size: u32,
}

impl TagHeader {
    pub fn tag_type(&self) -> Type {
        match self.tag_type {
            4 => Type::MemoryInfo,
            5 => Type::BiosBoot,
            1 => Type::BootCmdLine,
            3 => Type::Modules,
            9 => Type::ElfSymbols,
            6 => Type::MemoryMap,
            2 => Type::BootLoaderName,
            10 => Type::ApmTable,
            7 => Type::VbeInfo,
            8 => Type::FramebufferInfo,
            11 => Type::Efi32SystemTable,
            12 => Type::Efi64SystemTable,
            13 => Type::SMBiosTable,
            14 => Type::AcpiOldRdsp,
            15 => Type::AcpiNewRdsp,
            16 => Type::NetworkingInfo,
            17 => Type::EfiMemoryMap,
            18 => Type::EfiBootServicesNotTerminated,
            19 => Type::Efi32ImageHandle,
            20 => Type::Efi64ImageHandle,
            21 => Type::ImageLoadBase,
            _ => Type::Unknown,
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct MemoryInfo {
    header: TagHeader,
    mem_lower: u32,
    mem_upper: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct BiosBoot {
    header: TagHeader,
    bios_dev: u32,
    partition: u32,
    sub_partition: u32,
}

#[repr(C, packed)]
pub struct BootCmdLine {
    header: TagHeader,
    string: InternalCStr,
}

impl BootCmdLine {
    fn string(&self) -> &str {
        // SAFETY: This is safe, because we know the BootCmdLine tag will have an internal
        //         null-terminated UTF-8 string within the tag itself from the multiboot2 standard.
        unsafe { self.string.to_str() }
    }
}

#[repr(C, packed)]
pub struct Modules {
    header: TagHeader,
    mod_start: u32,
    mod_end: u32,
    string: InternalCStr,
}

impl Modules {
    fn string(&self) -> &str {
        // SAFETY: This is safe, because we know the Modules tag will have an internal
        //         null-terminated UTF-8 string within the tag itself from the multiboot2 standard.
        unsafe { self.string.to_str() }
    }
}

#[repr(C, packed)]
pub struct BootLoaderName {
    header: TagHeader,
    string: InternalCStr,
}

impl BootLoaderName {
    fn string(&self) -> &str {
        // SAFETY: This is safe, because we know the BootLoaderName tag will have an internal
        //         null-terminated UTF-8 string within the tag itself from the multiboot2 standard.
        unsafe { self.string.to_str() }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ApmTable {
    header: TagHeader,
    version: u16,
    cseg: u16,
    offset: u32,
    cseg_16: u16,
    dseg: u16,
    flags: u16,
    cseg_len: u16,
    cseg_16_len: u16,
    dseg_len: u16,
}

#[repr(C)]
pub struct VbeInfo {
    header: TagHeader,
    vbe_mode: u16,
    vbe_interface_seg: u16,
    vge_interface_off: u16,
    vbe_interface_len: u16,
    vbe_control_info: [u8; 512],
    vbe_mode_info: [u8; 256],
}

#[derive(Debug)]
#[repr(C)]
struct FramebufferInfo {
    // TODO
}

#[derive(Debug)]
#[repr(C)]
pub struct Efi32SystemTable {
    header: TagHeader,
    pointer: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct Efi64SystemTable {
    header: TagHeader,
    pointer: u64,
}

#[derive(Debug)]
#[repr(C)]
struct SMBiosTable {
    // TODO
}

#[derive(Debug)]
#[repr(C)]
pub struct AcpiOldRdsp {
    // TODO
}

#[derive(Debug)]
#[repr(C)]
pub struct AcpiNewRdsp {
    // TODO
}

#[derive(Debug)]
#[repr(C)]
pub struct NetworkingInfo {
    // TODO
}

#[derive(Debug)]
#[repr(C)]
pub struct EfiMemoryMap {
    // TODO
}

#[derive(Debug)]
#[repr(C)]
pub struct EfiBootServicesNotTerminated {
    // TODO
}

#[derive(Debug)]
#[repr(C)]
pub struct Efi32ImageHandle {
    // TODO
}

#[derive(Debug)]
#[repr(C)]
pub struct Efi64ImageHandle {
    // TODO
}

#[derive(Debug)]
#[repr(C)]
pub struct ImageLoadBase {
    // TODO
}
