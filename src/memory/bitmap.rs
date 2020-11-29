use core::mem::size_of;

use super::{FrameAllocatorImpl, PhysicalMemoryRegion, RawFrame, PAGE_SIZE};
use crate::arch::x86_64::paging::PhysicalAddress;

/// A simple "bootstrap" allocator. This uses a fixed-size, internal bitmap to track allocations,
/// and should only be used during early booting. This can be used to boostrap other, more
/// complex allocators.
pub struct BootstrapAllocatorImpl {
    bitmap: FixedBitmap,
    arena: PhysicalMemoryRegion,
}

impl BootstrapAllocatorImpl {
    fn first_frame_num(&self) -> usize {
        self.arena.base.align_up(PAGE_SIZE as u64).as_usize() / PAGE_SIZE
    }

    fn end_frame_num(&self) -> usize {
        self.arena.end().frame_num()
    }

    /// Converts a bitmap index into a frame number.
    fn frame_number(&self, index: usize) -> usize {
        let first_frame_num = self.arena.base.frame_num();
        index + first_frame_num
    }

    /// Allocates a sub-range from the allocator. Can be useful for creating
    /// sub-allocators. Currently, only supports 4K aligned sizes.
    fn alloc_range(&mut self, size: usize) -> Option<PhysicalMemoryRegion> {
        if size & 0xFFF != 0 {
            panic!("Trying to sub-allocate non-4k aligned size!")
        }

        let num_frames = size / PAGE_SIZE;
        let start_frame = self.bitmap.contiguous_range(num_frames)?;
        let first_frame_num = self.first_frame_num();

        // ensure frame doesn't go outside arena
        // We could probably make this faster by encoding size in the bitmap somehow
        if start_frame + num_frames + first_frame_num > self.end_frame_num() {
            return None;
        }

        // set all frames as allocated
        for index in start_frame..(start_frame + num_frames) {
            self.bitmap.set(index);
        }

        Some(PhysicalMemoryRegion {
            base: PhysicalAddress::from_frame_num(start_frame + first_frame_num),
            size: num_frames * PAGE_SIZE,
        })
    }
}

impl FrameAllocatorImpl for BootstrapAllocatorImpl {
    fn new() -> BootstrapAllocatorImpl {
        BootstrapAllocatorImpl {
            bitmap: FixedBitmap { words: [0; 64] },
            arena: PhysicalMemoryRegion::empty(),
        }
    }

    fn init(&mut self, region: PhysicalMemoryRegion) {
        self.arena = region;
    }

    fn alloc(&mut self) -> Option<RawFrame> {
        let first_frame_num = self.first_frame_num();
        let first_free = self.bitmap.first_unset()?;
        let frame_num = first_free + first_frame_num;

        // ensure frame doesn't go outside arena
        // We could probably make this faster by encoding size in the bitmap somehow
        if frame_num + 1 > self.end_frame_num() {
            return None;
        }

        self.bitmap.set(first_free);
        Some(RawFrame { num: frame_num })
    }

    fn dealloc(&mut self, frame: RawFrame) {
        let first_frame_num = self.first_frame_num();

        // TODO: is this the best way to error handle here?
        if frame.num < first_frame_num || frame.num > self.end_frame_num() {
            panic!("Attempting to free frame outside of arena!");
        }

        let bitmap_num = frame.num - first_frame_num;

        debug_assert!(
            self.bitmap.is_set(bitmap_num),
            "Attempting to free unallocated frame!"
        );

        self.bitmap.unset(bitmap_num);
    }
}

pub struct FixedBitmap {
    words: [u64; 64],
}

impl FixedBitmap {
    pub fn set(&mut self, index: usize) {
        let word_index = index / 64;
        let bit_offset = index % 64;
        self.words[word_index] |= 0x8000_0000_0000_0000 >> bit_offset;
    }

    pub fn unset(&mut self, index: usize) {
        let word_index = index / (size_of::<u64>() * 8);
        let bit_offset = index % (size_of::<u64>() * 8);
        self.words[word_index] &= !(0x8000_0000_0000_0000 >> bit_offset);
    }

    pub fn is_set(&self, index: usize) -> bool {
        let word_index = index / (size_of::<u64>() * 8);
        let bit_offset = index % (size_of::<u64>() * 8);
        (self.words[word_index] & (0x8000_0000_0000_0000 >> bit_offset)) != 0
    }

    fn first_unset(&self) -> Option<usize> {
        for i in 0..self.words.len() {
            let word = self.words[i];
            if self.words[i] != 0xFFFF_FFFF_FFFF_FFFF {
                return Some(i * 8 + word.leading_ones() as usize);
            }
        }

        return None;
    }

    /// Finds a contiguous range of free frames in the bitmap, and returns
    /// the index of the first frame.
    ///
    /// NOTE: This is VERY inefficient, and probably should not be used if it can be avoided,
    ///       as it must do a linear scan over all bits.
    fn contiguous_range(&self, num: usize) -> Option<usize> {
        if num > self.size() {
            return None;
        }

        let mut count = 0;
        for i in 0..self.words.len() {
            let mut word = self.words[i];
            for b in 0..(size_of::<u64>() * 8) {
                // if first bit not set
                if word & 0x8000_0000_0000_0000 == 0 {
                    count += 1;
                } else {
                    count = 0;
                }

                if count == num {
                    return Some((i * 8) + b + 1 - num);
                }

                word <<= 1;
            }
        }

        return None;
    }

    fn size(&self) -> usize {
        self.words.len() * 8
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::arch::x86_64::paging::PhysicalAddress;

    #[test]
    fn first_entry() {
        let mut bitmap = FixedBitmap { words: [0u64; 64] };

        bitmap.set(0);

        assert!(bitmap.is_set(0));
    }

    #[test]
    fn aligned_first() {
        let mut bitmap = FixedBitmap { words: [0u64; 64] };
        let region = PhysicalMemoryRegion::new(PhysicalAddress::new(0x1000), 0x5000);
        let mut bitmap_alloc = BootstrapAllocatorImpl {
            bitmap,
            arena: region,
        };

        assert_eq!(bitmap_alloc.alloc(), Some(RawFrame { num: 0x1 }));
    }

    #[test]
    fn set() {
        let mut bitmap = FixedBitmap { words: [0u64; 64] };

        bitmap.set(0);
        bitmap.set(20);
        bitmap.set(21);
        bitmap.set(300);
        bitmap.set(420);

        assert!(bitmap.is_set(0));
        assert!(bitmap.is_set(20));
        assert!(bitmap.is_set(21));
        assert!(bitmap.is_set(300));
        assert!(bitmap.is_set(420));
    }

    #[test]
    fn unset() {
        let mut bitmap = FixedBitmap { words: [0u64; 64] };

        bitmap.set(20);
        bitmap.set(21);
        bitmap.unset(20);

        assert!(!bitmap.is_set(20));
        assert!(bitmap.is_set(21));
    }

    #[test]
    fn range() {
        let mut bitmap = FixedBitmap { words: [0u64; 64] };

        bitmap.set(20);
        bitmap.set(21);
        bitmap.unset(20);

        assert_eq!(bitmap.contiguous_range(3), Some(0));
        bitmap.set(2);
        assert_eq!(bitmap.contiguous_range(3), Some(3));
        bitmap.set(4);
        bitmap.set(6);
        bitmap.set(10);
        assert_eq!(bitmap.contiguous_range(3), Some(7));
        bitmap.set(9);
        assert_eq!(bitmap.contiguous_range(3), Some(11));
    }

    #[test]
    fn next_free() {
        let mut bitmap = FixedBitmap { words: [0u64; 64] };

        assert_eq!(bitmap.first_unset(), Some(0));
        bitmap.set(0);
        assert_eq!(bitmap.first_unset(), Some(1));
        bitmap.set(6);
        assert_eq!(bitmap.first_unset(), Some(1));
        bitmap.set(1);
        bitmap.set(2);
        bitmap.set(3);
        bitmap.set(4);
        bitmap.set(6);
        bitmap.set(7);
        assert_eq!(bitmap.first_unset(), Some(5));
        bitmap.set(5);
        assert_eq!(bitmap.first_unset(), Some(8));
    }

    #[test]
    fn alloc() {
        let bitmap = FixedBitmap { words: [0u64; 64] };
        let region = PhysicalMemoryRegion::new(PhysicalAddress::new(0x1300), 0x5000);
        let mut bitmap_alloc = BootstrapAllocatorImpl {
            bitmap,
            arena: region,
        };

        assert_eq!(bitmap_alloc.alloc(), Some(RawFrame { num: 0x2 }));
        assert_eq!(bitmap_alloc.alloc(), Some(RawFrame { num: 0x3 }));
        assert_eq!(bitmap_alloc.alloc(), Some(RawFrame { num: 0x4 }));
        assert_eq!(bitmap_alloc.alloc(), Some(RawFrame { num: 0x5 }));
        assert_eq!(bitmap_alloc.alloc(), None);
        assert_eq!(bitmap_alloc.alloc(), None);

        bitmap_alloc.dealloc(RawFrame { num: 0x3 });
        bitmap_alloc.dealloc(RawFrame { num: 0x5 });
        assert_eq!(bitmap_alloc.alloc(), Some(RawFrame { num: 0x3 }));
        assert_eq!(bitmap_alloc.alloc(), Some(RawFrame { num: 0x5 }));
        assert_eq!(bitmap_alloc.alloc(), None);
    }

    #[test]
    fn alloc_sub_range() {
        let bitmap = FixedBitmap { words: [0u64; 64] };
        let region = PhysicalMemoryRegion::new(PhysicalAddress::new(0x1300), 0x5000);
        let mut bitmap_alloc = BootstrapAllocatorImpl {
            bitmap,
            arena: region,
        };

        assert_eq!(
            bitmap_alloc.alloc_range(4096),
            Some(PhysicalMemoryRegion {
                base: PhysicalAddress::new(0x2000),
                size: 4096
            })
        );
        assert_eq!(
            bitmap_alloc.alloc_range(8192),
            Some(PhysicalMemoryRegion {
                base: PhysicalAddress::new(0x3000),
                size: 8192
            })
        );
        assert_eq!(bitmap_alloc.alloc(), Some(RawFrame { num: 5 }));
        assert_eq!(bitmap_alloc.alloc(), None);
        assert_eq!(bitmap_alloc.alloc_range(4096), None);
        assert_eq!(bitmap_alloc.alloc_range(8192), None);
    }

    #[test]
    #[should_panic(expected = "Attempting to free unallocated frame!")]
    fn dealloc_unallocated() {
        let bitmap = FixedBitmap { words: [0u64; 64] };
        let region = PhysicalMemoryRegion::new(PhysicalAddress::new(0x1300), 0x5000);
        let mut bitmap_alloc = BootstrapAllocatorImpl {
            bitmap,
            arena: region,
        };

        bitmap_alloc.dealloc(RawFrame { num: 3 });
    }

    #[test]
    #[should_panic(expected = "Attempting to free frame outside of arena!")]
    fn dealloc_outside_arena() {
        let bitmap = FixedBitmap { words: [0u64; 64] };
        let region = PhysicalMemoryRegion::new(PhysicalAddress::new(0x1300), 0x5000);
        let mut bitmap_alloc = BootstrapAllocatorImpl {
            bitmap,
            arena: region,
        };

        bitmap_alloc.dealloc(RawFrame { num: 12 });
    }
}
