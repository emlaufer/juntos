//! Contains the architecture independent portions of the memory manager
//! Some of this was inspired by Redox, others inspired by Phil OS

mod bitmap;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::paging::{PhysicalAddress, PAGE_SIZE};
use crate::println;
pub use bitmap::BootstrapAllocatorImpl;

// TODO: If allocator needs some args to init, we can add that.
//       Though for now this should be fine.
macro_rules! frame_allocator {
    ($type:tt, $impl:ty) => {
        #[derive(Debug, Copy, Clone)]
        pub struct $type;

        impl $type {
            fn __impl() -> &'static Mutex<$impl> {
                lazy_static! {
                    static ref ALLOCATOR: Mutex<$impl> = Mutex::new(<$impl>::new());
                }
                &ALLOCATOR
            }
        }

        unsafe impl FrameAllocator for $type {
            unsafe fn init(region: PhysicalMemoryRegion) {
                <$type>::__impl().lock().init(region);
            }

            fn get() -> Self {
                $type
            }

            fn alloc(&self) -> Option<Frame<Self>> {
                <$type>::__impl().lock().alloc().map(|frame| Frame {
                    num: frame.num,
                    alloc: $type,
                })
            }

            #[doc(hidden)]
            unsafe fn __free_frame(&self, frame: &mut Frame<Self>) {
                println!("We are freeing frame");
                <$type>::__impl()
                    .lock()
                    .dealloc(RawFrame { num: frame.num })
            }
        }
    };
}

frame_allocator!(BootstrapAllocator, BootstrapAllocatorImpl);

/// Represents a handle to a static FrameAllocator. It should only be implemented using the
/// frame_allocator macro.
pub unsafe trait FrameAllocator: Copy {
    /// Initializes the memory allocator by giving it a memory region to manage.
    /// This method consumes the region, i.e. currently this is a permanent decision.
    ///
    /// # Safety
    /// This method is unsafe, because it could be used to leak physical memory if called
    /// multiple times. Therefore, it is safe so long as it is only called once.
    unsafe fn init(region: PhysicalMemoryRegion);

    /// Returns a handle to the memory allocator.
    fn get() -> Self;

    /// Allocates a frame from the memory allocator. This frame will automatically be freed when
    /// it is dropped.
    fn alloc(&self) -> Option<Frame<Self>>;

    #[doc(hidden)]
    unsafe fn __free_frame(&self, f: &mut Frame<Self>);
}

#[derive(Debug)]
pub struct Frame<A: FrameAllocator> {
    alloc: A, // ZST to allocator
    num: usize,
}

impl<A> Frame<A>
where
    A: FrameAllocator,
{
    pub fn num(&self) -> usize {
        self.num
    }

    pub fn addr(&self) -> PhysicalAddress {
        PhysicalAddress::from(self.num * PAGE_SIZE)
    }

    pub fn containing(addr: usize) -> Frame<BootstrapAllocator> {
        Frame::<BootstrapAllocator> {
            alloc: BootstrapAllocator,
            num: addr / PAGE_SIZE,
        }
    }
}

impl<A: FrameAllocator> Drop for Frame<A> {
    fn drop(&mut self) {
        let alloc = self.alloc;
        unsafe { alloc.__free_frame(self) };
    }
}

/// The methods that a FrameAllocator implementation must implement. Any instance of this
/// should not be created manually, and only through the frame_allocator! macro.
pub trait FrameAllocatorImpl {
    fn new() -> Self;
    fn init(&mut self, arena: PhysicalMemoryRegion);
    fn alloc(&mut self) -> Option<RawFrame>;
    fn dealloc(&mut self, frame: RawFrame);
}

// TODO: associate a frame with its allocator,
//       automatically free it when dropped.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RawFrame {
    pub num: usize,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MemoryRange {
    start_addr: PhysicalAddress,
    end_addr: PhysicalAddress,
}

impl MemoryRange {
    // This is okay to be public correct? Yes because it doesn't really mean much?
    // Or no because we can create a memory allocator with invalid regions?
    pub fn new(start_addr: usize, end_addr: usize) -> MemoryRange {
        assert!(start_addr <= end_addr);

        MemoryRange {
            start_addr: PhysicalAddress::from(start_addr),
            end_addr: PhysicalAddress::from(end_addr),
        }
    }

    /// Returns true if this region entirely contains `region`
    pub fn contains(&self, region: &MemoryRange) -> bool {
        self.start_addr <= region.start_addr && region.end_addr <= self.end_addr
    }
}
