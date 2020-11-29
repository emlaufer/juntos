pub mod tag;

use core::marker::PhantomData;

use crate::memory::MemoryRange;
use tag::*;

// TODO: I may want to wrap this in another struct
pub struct Multiboot2Info<'a> {
    total_size: u32,
    _reserved: u32,
    _marker: PhantomData<&'a TagHeader>,
}

impl<'a> Multiboot2Info<'a> {
    fn tags(&self) -> TagIterator {
        // SAFETY: This is safe because the multiboot2 standard ensures the first tag will always
        //         immediatly follow this header. This ptr will also be non-null and 8-byte aligned
        //         as the header starts 8-byte aligned and is 8 bytes big.
        unsafe { TagIterator::new((self as *const Multiboot2Info).offset(1) as *const TagHeader) }
    }

    /// Returns a logical memory region in which this multiboot2 struct resides
    pub fn memory_region(&self) -> MemoryRange {
        let start = (self as *const Multiboot2Info) as usize;
        MemoryRange::new(start, start + self.total_size as usize)
    }

    pub fn memory_info(&self) -> Option<&'a MemoryInfo> {
        // SAFETY: This is safe, as we know the TagHeader is valid from the tag iterator, and we
        //         also know from the multiboot2 standard that the tag with type 4 is a valid
        //         MemoryInfo tag.
        self.tags()
            .find(|tag| tag.tag_type == 4)
            .map(|header| unsafe { &*((header as *const TagHeader) as *const MemoryInfo) })
    }

    pub fn memory_map(&self) -> Option<&'a MemoryMap> {
        // SAFETY: This is safe, as we know the TagHeader is valid from the tag iterator, and we
        //         also know from the multiboot2 standard that the tag with type 6 is a valid
        //         MemoryMap tag.
        self.tags()
            .find(|tag| tag.tag_type == 6)
            .map(|header| unsafe { &*((header as *const TagHeader) as *const MemoryMap) })
    }

    pub fn elf_symbols(&self) -> Option<&'a ElfSymbols> {
        // SAFETY: This is safe, as we know the TagHeader is valid from the tag iterator, and we
        //         also know from the multiboot2 standard that the tag with type 9 is a valid
        //         ElfSymbols tag.
        self.tags()
            .find(|tag| tag.tag_type == 9)
            .map(|header| unsafe { &*((header as *const TagHeader) as *const ElfSymbols) })
    }
}
