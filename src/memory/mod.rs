//! Contains the architecture independent portions of the memory manager
//! Some of this was inspired by Redox, others inspired by Phil OS

mod bitmap;
mod bump;

use core::cmp::{max, min};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::paging::{PhysicalAddress, PAGE_SIZE};
use crate::println;
pub use bitmap::{Bitmap, BitmapAllocator};
pub use bump::BumpAllocator;

pub trait FrameAllocator {
    fn alloc(&mut self) -> Option<Frame>;
    fn dealloc(&mut self, frame: Frame);
}

// TODO: associate a frame with its allocator,
//       automatically free it when dropped.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Frame {
    pub num: usize,
}

impl Frame {
    pub fn addr_from_number(num: usize) -> PhysicalAddress {
        PhysicalAddress::from_usize(num * PAGE_SIZE)
    }

    pub fn number_from_addr(addr: PhysicalAddress) -> usize {
        addr.as_usize() / PAGE_SIZE
    }

    pub fn addr(&self) -> PhysicalAddress {
        Frame::addr_from_number(self.num)
    }

    pub fn containing(addr: usize) -> Frame {
        Frame {
            num: addr / PAGE_SIZE,
        }
    }

    /// returns the first frame after a particular address
    fn after(addr: usize) -> Frame {
        Frame {
            num: (addr + PAGE_SIZE - 1) / PAGE_SIZE,
        }
    }

    // SUPER PRIVATE - We cannot allow consumers of this interface to create or copy their own frames!
    fn clone(&self) -> Frame {
        Frame { num: self.num }
    }

    // THIS needs to be SUPER private
    fn offset(&self, offset: isize) -> Frame {
        // ensure the frame number doesn't over or underflow
        // TODO: should we panic if it does? Probably
        let num = if offset.is_negative() {
            self.num
                .checked_sub(offset.wrapping_abs() as usize)
                .unwrap()
        } else {
            self.num.checked_add(offset as usize).unwrap()
        };

        Frame { num }
    }
}

struct FrameIter {
    current: Frame,
    end: Frame,
}

impl Iterator for FrameIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        if self.current <= self.end {
            let res = self.current.clone();
            self.current = self.current.offset(1);
            Some(res)
        } else {
            None
        }
    }
}

use crate::multiboot::tag::memory_map;

/// Represents an OWNED chunk of physical memory.
#[derive(Debug, Eq, PartialEq)]
pub struct PhysicalMemoryRegion {
    pub base: PhysicalAddress,
    pub size: usize,
}

impl PhysicalMemoryRegion {
    /// Returns an empty region. This is safe, because by definition
    /// it controls no memory.
    // TODO: should this be public
    pub const fn empty() -> PhysicalMemoryRegion {
        PhysicalMemoryRegion {
            base: PhysicalAddress::new(0),
            size: 0,
        }
    }

    #[cfg(test)]
    pub fn new(base: PhysicalAddress, size: usize) -> PhysicalMemoryRegion {
        PhysicalMemoryRegion { base, size }
    }

    // TODO: this should consume the entry
    pub fn from_multiboot(entry: &memory_map::Entry) -> PhysicalMemoryRegion {
        PhysicalMemoryRegion {
            base: PhysicalAddress::new(entry.base_addr),
            size: entry.length as usize,
        }
    }

    /// Removes the specific number of bytes from the begining of the region, and returns it
    /// as a new region.
    pub fn take(&mut self, amount: usize) -> PhysicalMemoryRegion {
        let old_base = self.base;
        self.base = self.base.add(amount as u64);
        self.size -= amount;

        PhysicalMemoryRegion {
            base: old_base,
            size: amount,
        }
    }

    pub fn end(&self) -> PhysicalAddress {
        self.base.add(self.size as u64)
    }
}

/// Represents a
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MemoryRegion {
    pub start_addr: usize,
    pub end_addr: usize,
}

impl MemoryRegion {
    // This is okay to be public correct? Yes because it doesn't really mean much?
    // Or no because we can create a memory allocator with invalid regions?
    pub fn new(start_addr: usize, end_addr: usize) -> MemoryRegion {
        assert!(start_addr <= end_addr);

        MemoryRegion {
            start_addr,
            end_addr,
        }
    }

    fn start_frame(&self) -> Frame {
        // get the first FULL frame
        // NOTE: cannot just use `Frame::containing(self.start_addr - 1)` in case `start_addr` is 0
        Frame::after(self.start_addr)
    }

    fn end_frame(&self) -> Frame {
        // get the last FULL frame
        // End is exclusive, so we always want the frame before
        Frame::containing(self.end_addr).offset(-1)
    }

    /// Returns true if this region entirely contains `region`
    pub fn contains(&self, region: &MemoryRegion) -> bool {
        self.start_addr <= region.start_addr && region.end_addr <= self.end_addr
    }

    /// Returns a MemoryRegion that is the intersection of both regions
    pub fn intersection(&self, region: &MemoryRegion) -> Option<MemoryRegion> {
        let start = max(self.start_addr, region.start_addr);
        let end = min(self.end_addr, region.end_addr);

        if start >= end {
            None
        } else {
            Some(MemoryRegion::new(start, end))
        }
    }

    #[allow(unused)]
    fn contains_frame(&self, frame: &Frame) -> bool {
        self.start_frame() <= *frame && *frame <= self.end_frame()
    }

    /// Subtracts one memory region from another, returning
    pub fn subtract(self, other: MemoryRegion) -> (MemoryRegion, Option<MemoryRegion>) {
        // kernel entirely inside region
        if self.start_addr < other.start_addr && other.end_addr < self.end_addr {
            (
                MemoryRegion::new(self.start_addr, other.start_addr),
                Some(MemoryRegion::new(other.end_addr, self.end_addr)),
            )
        // kernel is at start and overlapping
        } else if other.start_addr <= self.start_addr
            && self.start_addr <= other.end_addr
            && other.end_addr <= self.end_addr
        {
            (MemoryRegion::new(other.end_addr, self.end_addr), None)
        // kernel at end and overlapping
        } else if self.start_addr < other.start_addr
            && other.start_addr < self.end_addr
            && self.end_addr <= other.end_addr
        {
            (MemoryRegion::new(self.start_addr, other.start_addr), None)
        } else {
            (self, None)
        }
    }

    fn frames(&self) -> FrameIter {
        FrameIter {
            current: self.start_frame(),
            end: self.end_frame(),
        }
    }
}
