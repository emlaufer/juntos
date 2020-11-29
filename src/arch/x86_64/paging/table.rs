use bitflags::bitflags;
use core::fmt;
use core::ops::{Index, IndexMut};

use super::addr::PhysicalAddress;
use crate::memory::{Frame, FrameAllocator};

/// Represents a page table within a recursive tree
#[repr(C)]
pub struct RecursivePageTable {
    entries: [Entry; 512],
}

impl RecursivePageTable {
    /// Gets the next level page table at a specific index
    ///
    /// # Safety
    /// This is only safe if this exists within the active page table tree, and the
    /// L4 table is recursive, i.e. the last entry points to itself.
    pub unsafe fn get_table(&self, index: usize) -> Option<&RecursivePageTable> {
        if self[index].is_present() {
            Some(&*self.get_table_ptr(index))
        } else {
            None
        }
    }

    /// Gets the next level page table at a specific index
    ///
    /// # Safety
    /// This is only safe if this exists within the active page table tree, and the
    /// L4 table is recursive, i.e. the last entry points to itself.
    pub unsafe fn get_table_mut(&mut self, index: usize) -> Option<&mut RecursivePageTable> {
        if self[index].is_present() {
            Some(&mut *self.get_table_ptr(index))
        } else {
            None
        }
    }

    pub unsafe fn create_table<A>(&mut self, index: usize, alloc: A) -> &mut RecursivePageTable
    where
        A: FrameAllocator,
    {
        // if entry not present create an entry
        if !self[index].is_present() {
            let frame = alloc
                .alloc()
                .expect("Out of memory for creating page tables!");
            self[index] = Entry::new(&frame, Flags::PRESENT | Flags::WRITE | Flags::USER);

            // TODO: Should formalize this better
            core::mem::forget(frame);
            self.get_table_mut(index)
                .expect("Table entry after allocation still empty!")
                .clear();
        }

        self.get_table_mut(index)
            .expect("Table entry after allocation still empty!")
    }
}

impl RecursivePageTable {
    fn get_table_ptr(&self, index: usize) -> *mut RecursivePageTable {
        ((((self as *const _ as usize) >> 3) | index as usize) << 12) as *mut _
    }

    fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.clear();
        }
    }
}

impl Index<usize> for RecursivePageTable {
    type Output = Entry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for RecursivePageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

#[derive(Clone, Copy)]
pub struct Entry(u64);

impl Entry {
    const ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

    pub fn new<A>(frame: &Frame<A>, flags: Flags) -> Entry
    where
        A: FrameAllocator,
    {
        Entry(((frame.num() as u64) << 12) | flags.bits)
    }

    pub fn empty() -> Entry {
        Entry(0)
    }

    pub fn clear(&mut self) {
        self.0 = 0
    }

    pub fn addr(&self) -> PhysicalAddress {
        // bottom 12 bits are 0, cause pages must be 4k aligned
        PhysicalAddress::new(self.0 & Entry::ADDR_MASK)
    }

    fn flags(&self) -> Flags {
        Flags::from_bits_truncate(self.0)
    }

    pub fn is_present(&self) -> bool {
        // TODO: For now, just assume an entry that is present is valid
        // We will need to make sure this is always the case I guess
        self.flags().contains(Flags::PRESENT)
    }
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Entry {{ addr: {:?}, flags: {:?} }}",
            self.addr(),
            self.flags()
        )
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Flags: u64 {
        const PRESENT = 1 << 0;
        const WRITE = 1 << 1;
        const USER = 1 << 2;
        const PAGE_WRITETHROUGH = 1 << 3;
        const NO_CACHE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const PAT = 1 << 7;
        const GLOBAL = 1 << 8;
        const NO_EXECUTE = 1 << 63;
    }
}
