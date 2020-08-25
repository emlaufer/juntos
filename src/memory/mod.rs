//! Contains the architecture independent portions of the memory manager
//! Some of this was inspired by Redox, others inspired by Phil OS

mod bump;

use core::cmp::{max, min};

use crate::arch::paging::PAGE_SIZE;
pub use bump::BumpAllocator;

pub trait FrameAllocator {
    fn alloc(&mut self) -> Option<Frame>;
    fn dealloc(&mut self, frame: Frame);
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Frame {
    num: usize,
}

impl Frame {
    fn containing(addr: usize) -> Frame {
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

/// Simply represents some contiguous region in memory
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
