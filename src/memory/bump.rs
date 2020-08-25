/// A simple bump allocator for physical frames
/// TODO
use super::{Frame, FrameAllocator, FrameIter, MemoryRegion};

pub struct BumpAllocator<I: Iterator<Item = MemoryRegion>> {
    frames_iter: Option<FrameIter>,
    free_regions: I,
}

impl<I: Iterator<Item = MemoryRegion>> BumpAllocator<I> {
    // TODO: should this copy? probably easier to not
    pub fn new(mut free_regions: I) -> BumpAllocator<I> {
        BumpAllocator {
            frames_iter: free_regions.next().map(|region| region.frames()),
            free_regions,
        }
    }
}

impl<I: Iterator<Item = MemoryRegion>> FrameAllocator for BumpAllocator<I> {
    fn alloc(&mut self) -> Option<Frame> {
        // returns None if frames_iter is none, else get next frame
        let frame = self.frames_iter.as_mut()?.next();

        // if frame is none, then increment frames iterator and recurse
        if frame.is_none() {
            self.frames_iter = self.free_regions.next().map(|region| region.frames());
            self.alloc()
        } else {
            frame
        }
    }

    fn dealloc(&mut self, _frame: Frame) {
        unimplemented!();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn alloc() {
        let regions = vec![
            MemoryRegion::new(0x100, 0x10000),
            MemoryRegion::new(0x10010, 0x10020), // too small, make sure no frames!
            MemoryRegion::new(0x20000, 0x50000),
        ];

        let mut allocator = BumpAllocator::new(regions.clone().into_iter());

        while let Some(frame) = allocator.alloc() {
            assert!(regions.iter().any(|region| region.contains_frame(&frame)));
            assert!(!regions[1].contains_frame(&frame))
        }
    }
}
