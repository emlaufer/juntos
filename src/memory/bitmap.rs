use core::mem::size_of;
use core::ptr::Unique;

use super::Frame;
use super::FrameAllocator;
use super::PhysicalMemoryRegion;
use super::PAGE_SIZE;

pub struct Bitmap<'a> {
    pub bits: &'a mut [u64],
}

impl<'a> Bitmap<'a> {
    /*pub fn empty() -> Bitmap {
        Bitmap { bits: [] }
    }*/

    pub fn new(mem: &'a mut [u64]) -> Bitmap {
        for item in mem.iter_mut() {
            *item = 0;
        }

        Bitmap { bits: mem }
    }

    pub fn set(&mut self, index: usize) {
        let word_index = index / 64;
        let bit_offset = index % 64;
        self.bits[word_index] |= 0x8000_0000_0000_0000 >> bit_offset;
    }

    pub fn unset(&mut self, index: usize) {
        let word_index = index / (size_of::<u64>() * 8);
        let bit_offset = index % (size_of::<u64>() * 8);
        self.bits[word_index] &= !(0x8000_0000_0000_0000 >> bit_offset);
    }

    pub fn is_set(&self, index: usize) -> bool {
        let word_index = index / (size_of::<u64>() * 8);
        let bit_offset = index % (size_of::<u64>() * 8);
        self.bits[word_index] & !(0x8000_0000_0000_0000 >> bit_offset) != 0
    }

    fn first_unset(&self) -> Option<usize> {
        for i in 0..self.len_words() {
            let word = self.get_word(i);
            if self.get_word(i) != 0xFFFF_FFFF_FFFF_FFFF {
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
        for i in 0..self.len_words() {
            let mut word = self.get_word(i);
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

    fn get_word(&self, index: usize) -> u64 {
        self.bits[index]
    }

    fn len_words(&self) -> usize {
        self.bits.len()
    }

    fn size(&self) -> usize {
        self.bits.len() * 8
    }
}

// For now, this allocator will manage all of
// memory.
pub struct BitmapAllocator<'a> {
    bitmap: Bitmap<'a>,
    arena: PhysicalMemoryRegion,
}

impl<'a> BitmapAllocator<'a> {
    /// Creates a new BitmapAllocator.
    pub fn new(bitmap: Bitmap, arena: PhysicalMemoryRegion) -> BitmapAllocator {
        // TODO: Check size of bitmap
        BitmapAllocator { bitmap, arena }
    }

    fn first_frame_num(&self) -> usize {
        self.arena.base.align_up(PAGE_SIZE as u64).as_usize() / PAGE_SIZE
    }

    fn end_frame_num(&self) -> usize {
        Frame::number_from_addr(self.arena.end())
    }

    /// Converts a bitmap index into a frame number.
    fn frame_number(&self, index: usize) -> usize {
        let first_frame_num = Frame::number_from_addr(self.arena.base);
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
            base: Frame::addr_from_number(start_frame + first_frame_num),
            size: num_frames * PAGE_SIZE,
        })
    }
}

impl<'a> FrameAllocator for BitmapAllocator<'a> {
    fn alloc(&mut self) -> Option<Frame> {
        let first_frame_num = self.first_frame_num();
        let first_free = self.bitmap.first_unset()?;
        let frame_num = first_free + first_frame_num;

        // ensure frame doesn't go outside arena
        // We could probably make this faster by encoding size in the bitmap somehow
        if frame_num + 1 > self.end_frame_num() {
            return None;
        }

        self.bitmap.set(first_free);
        Some(Frame { num: frame_num })
    }

    fn dealloc(&mut self, frame: Frame) {
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::arch::x86_64::paging::PhysicalAddress;

    #[test]
    fn first_entry() {
        let mut mem = [0 as u64; 20];

        let mut bitmap = Bitmap { bits: &mut mem };

        bitmap.set(0);

        assert_eq!(mem[0], 0x8000_0000_0000_0000);
    }

    #[test]
    fn aligned_first() {
        let mut mem = [0 as u64; 20];
        let bitmap = Bitmap { bits: &mut mem };
        let region = PhysicalMemoryRegion::new(PhysicalAddress::new(0x1000), 0x5000);
        let mut bitmap_alloc = BitmapAllocator::new(bitmap, region);

        assert_eq!(bitmap_alloc.alloc(), Some(Frame { num: 0x1 }));
    }

    #[test]
    fn set() {
        let mut mem = [0 as u64; 20];

        let mut bitmap = Bitmap { bits: &mut mem };

        bitmap.set(0);
        bitmap.set(20);
        bitmap.set(21);
        bitmap.set(300);
        bitmap.set(420);

        assert_eq!(mem[0], 0x8000_0C00_0000_0000);
        assert_eq!(mem[4], 2u64.pow(19));
        assert_eq!(mem[6], 2u64.pow(27));
    }

    #[test]
    fn unset() {
        let mut mem = [0 as u64; 20];
        let mut bitmap = Bitmap { bits: &mut mem };

        bitmap.set(20);
        bitmap.set(21);
        bitmap.unset(20);

        assert_eq!(mem[0], 0x0000_0400_0000_0000);
    }

    #[test]
    fn range() {
        let mut mem = [0 as u64; 20];
        let mut bitmap = Bitmap { bits: &mut mem };

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
        let mut mem = [0 as u64; 20];
        let mut bitmap = Bitmap { bits: &mut mem };

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
        let mut mem = [0 as u64; 20];
        let bitmap = Bitmap { bits: &mut mem };
        let region = PhysicalMemoryRegion::new(PhysicalAddress::new(0x1300), 0x5000);
        let mut bitmap_alloc = BitmapAllocator::new(bitmap, region);

        assert_eq!(bitmap_alloc.alloc(), Some(Frame { num: 0x2 }));
        assert_eq!(bitmap_alloc.alloc(), Some(Frame { num: 0x3 }));
        assert_eq!(bitmap_alloc.alloc(), Some(Frame { num: 0x4 }));
        assert_eq!(bitmap_alloc.alloc(), Some(Frame { num: 0x5 }));
        assert_eq!(bitmap_alloc.alloc(), None);
        assert_eq!(bitmap_alloc.alloc(), None);

        bitmap_alloc.dealloc(Frame { num: 0x3 });
        bitmap_alloc.dealloc(Frame { num: 0x5 });
        assert_eq!(bitmap_alloc.alloc(), Some(Frame { num: 0x3 }));
        assert_eq!(bitmap_alloc.alloc(), Some(Frame { num: 0x5 }));
        assert_eq!(bitmap_alloc.alloc(), None);
    }

    #[test]
    fn alloc_sub_range() {
        let mut mem = [0 as u64; 20];
        let bitmap = Bitmap { bits: &mut mem };
        let region = PhysicalMemoryRegion::new(PhysicalAddress::new(0x1300), 0x5000);
        let mut bitmap_alloc = BitmapAllocator::new(bitmap, region);

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
        assert_eq!(bitmap_alloc.alloc(), Some(Frame { num: 5 }));
        assert_eq!(bitmap_alloc.alloc(), None);
        assert_eq!(bitmap_alloc.alloc_range(4096), None);
        assert_eq!(bitmap_alloc.alloc_range(8192), None);
    }

    #[test]
    #[should_panic(expected = "Attempting to free unallocated frame!")]
    fn dealloc_unallocated() {
        let mut mem = [0 as u64; 20];
        let bitmap = Bitmap { bits: &mut mem };
        let region = PhysicalMemoryRegion::new(PhysicalAddress::new(0x1300), 0x5000);
        let mut bitmap_alloc = BitmapAllocator::new(bitmap, region);

        bitmap_alloc.dealloc(Frame { num: 3 });
    }

    #[test]
    #[should_panic(expected = "Attempting to free frame outside of arena!")]
    fn dealloc_outside_arena() {
        let mut mem = [0 as u64; 20];
        let bitmap = Bitmap { bits: &mut mem };
        let region = PhysicalMemoryRegion::new(PhysicalAddress::new(0x1300), 0x5000);
        let mut bitmap_alloc = BitmapAllocator::new(bitmap, region);

        bitmap_alloc.dealloc(Frame { num: 12 });
    }
}
