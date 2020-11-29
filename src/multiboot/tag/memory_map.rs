use core::marker::PhantomData;
use core::mem::size_of;

use super::TagHeader;

#[derive(Debug)]
#[repr(C)]
pub struct MemoryMap<'a> {
    header: TagHeader,
    entry_size: u32,
    entry_version: u32,
    _marker: PhantomData<&'a [Entry]>, // entries are after this tag
}

impl<'a> MemoryMap<'a> {
    pub fn entries(&self) -> Iter<'a> {
        // SAFETY: This is safe because `self.offset(1)` will return the first byte past the
        //         MemoryMap struct in memory, the computed offset cannot overflow an isize, and
        //         the offset will not wrap around the address space (because the structs are populated
        //         by the multiboot2 header to fit in physical memory)
        let list_ptr = unsafe { (self as *const MemoryMap).offset(1) as *const Entry };
        let entries_remaining =
            (self.header.size as usize - size_of::<MemoryMap>()) / self.entry_size as usize;

        Iter {
            current_entry: list_ptr,
            entry_size: self.entry_size,
            entries_remaining,
            _marker: PhantomData,
        }
    }

    pub fn available(&self) -> impl Iterator<Item = &'a Entry> + 'a {
        self.entries().filter_map(|entry| {
            if entry.entry_type() == EntryType::Available {
                Some(entry)
            } else {
                None
            }
        })
    }
}

// To ensure forward compatibility, we need to ensure we always move forward by entry_size
pub struct Iter<'a> {
    current_entry: *const Entry,
    entry_size: u32,
    entries_remaining: usize,
    _marker: PhantomData<&'a Entry>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Entry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.entries_remaining == 0 {
            return None;
        }

        // SAFETY: We now that current_entry will be a non-null because there are entries left, which
        //         is calculated from multiboot2 information we can assume is correct. It is aligned, because the entries
        //         start after a memory map tag, whose size is 16 bytes and is also 8 byte aligned.
        //         We also know it points to a valid `Entry` struct, as that is defined by the
        //         multiboot2 standard.
        let entry = unsafe { &*self.current_entry };
        self.current_entry =
            (self.current_entry as usize + self.entry_size as usize) as *const Entry;
        self.entries_remaining -= 1;

        Some(entry)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum EntryType {
    Reserved,
    Available,
    Acpi,
    PreservedHibernation,
    Defective,
}

#[derive(Debug)]
#[repr(C)]
pub struct Entry {
    pub base_addr: u64,
    pub length: u64,
    entry_type: u32,
    _reserved: u32,
}

impl Entry {
    pub fn start_addr(&self) -> u64 {
        self.base_addr
    }

    pub fn end_addr(&self) -> u64 {
        self.base_addr + self.length
    }

    pub fn entry_type(&self) -> EntryType {
        match self.entry_type {
            1 => EntryType::Available,
            3 => EntryType::Acpi,
            4 => EntryType::PreservedHibernation,
            5 => EntryType::Defective,
            _ => EntryType::Reserved,
        }
    }
}
