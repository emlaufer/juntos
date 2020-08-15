use bitflags::bitflags;
use core::mem::size_of;

use super::super::PriviledgeLevel;
use super::Tss;

// TODO: Is there any specific number of entries we should have? Lets just use 16 for now (some
// null)
// TODO: Ideally, we could do some compile time template magic to size the struct based on what
//       it needs to be.
const GDT_SIZE: usize = 16;

#[repr(C, packed)]
pub struct Gdt {
    entries: [Descriptor; GDT_SIZE],
    index: usize,
}

impl Gdt {
    pub fn new() -> Gdt {
        Gdt {
            entries: [Descriptor::new(0, 0, 0, Flags::default()); GDT_SIZE],
            index: 1,
        }
    }

    pub fn add_code_segment(&mut self, base: u32, limit: u32) -> SegmentSelector {
        let access = AccessFlags::PRESENT
            | AccessFlags::CODE_OR_DATA
            | AccessFlags::EXECUTABLE
            | AccessFlags::READ_WRITE;

        self.add_entry(Descriptor::new(
            base,
            limit,
            access.bits,
            Flags::LONG_MODE | Flags::PAGE_GRANULARITY,
        ))
    }

    pub fn add_data_segment(&mut self, base: u32, limit: u32) -> SegmentSelector {
        let access = AccessFlags::PRESENT | AccessFlags::CODE_OR_DATA | AccessFlags::READ_WRITE;
        self.add_entry(Descriptor::new(base, limit, access.bits, Flags::default()))
    }

    pub fn add_tss(&mut self, tss: &Tss) -> SegmentSelector {
        let tss_addr = (tss as *const _) as u64;

        // System Segments dont use the same access flags.
        // These are correct for a 64-bit TSS.
        // If we use call gates, we may want to introduce a new struct to wrap this
        let access = 0b10001001;

        let segment = self.add_entry(Descriptor::new(
            tss_addr as u32,
            (size_of::<Tss>() - 1) as u32,
            access,
            Flags::default(),
        ));

        // TSS entries are double width, and also hold the upper 32 bits of the tss addr
        self.add_entry(Descriptor::raw(tss_addr >> 32));

        segment
    }

    fn add_entry(&mut self, descriptor: Descriptor) -> SegmentSelector {
        self.entries[self.index] = descriptor;
        let selector = SegmentSelector((self.index * size_of::<Descriptor>()) as u16);
        self.index += 1;

        selector
    }

    /// ## Safety: The caller must ensure that `self` is a valid GDT, and that it will continue to live
    ///            as long as it is needed (i.e. it may not live on the stack). This also DOES NOT
    ///            load the segment registers. Those must be set or the new GDT will not be used.
    pub unsafe fn load(&self) {
        let ptr = GdtPseudoDescriptor {
            limit: (size_of::<Gdt>() - 1) as u16,
            base: self as *const _ as u64,
        };

        asm!("lgdt [{}]", in(reg) &ptr)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SegmentSelector(pub u16);

#[repr(C, packed)]
struct GdtPseudoDescriptor {
    limit: u16,
    base: u64,
}

/// Represents an entry into the GDT.
/// The format for code or data segments is specified as:
/// ```
///      3                   2                   1                  
///    1 0 9 8 7 6 5 4 3 2 1 0 9 8 7 6 5 4 3 2 1 0 9 8 7 6 5 4 3 2 1 0
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |        Base Addr[15:0]        |          Limit[15:0]          |  +0
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |  Base[31:24]  | Flags | Limit |     Access    |  Base  Addr   |  +4
///   |               |       |[19:16]|               |   [23:16]     |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
///
/// The format for system segments (i.e. for a TSS) is specified as:
/// ```
///      3                   2                   1                  
///    1 0 9 8 7 6 5 4 3 2 1 0 9 8 7 6 5 4 3 2 1 0 9 8 7 6 5 4 3 2 1 0
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |        Base Addr[15:0]        |          Limit[15:0]          |  +0
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |  Base[31:24]  | Flags | Limit |     Access    |  Base  Addr   |  +4
///   |               |       |[19:16]|               |   [23:16]     |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |                         Base[63:32]                           |  +8
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |                          Reserved                             |  +12
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
///
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
struct Descriptor(u64);

impl Descriptor {
    fn new(base: u32, limit: u32, access: u8, flags: Flags) -> Descriptor {
        let mut desc = Descriptor(0);

        desc.set_base(base)
            .set_limit(limit)
            .set_access(access)
            .set_flags(flags);

        desc
    }

    fn raw(bits: u64) -> Descriptor {
        Descriptor(bits)
    }

    fn set_base(&mut self, base: u32) -> &mut Descriptor {
        self.0 |= ((base & 0x00FFFFFF) as u64) << 16;
        self.0 |= ((base & 0xFF000000) as u64) << 32;
        self
    }

    fn set_limit(&mut self, limit: u32) -> &mut Descriptor {
        self.0 |= (limit & 0x0000FFFF) as u64;
        self.0 |= ((limit & 0x000F0000) as u64) << 32;
        self
    }

    fn set_flags(&mut self, flags: Flags) -> &mut Descriptor {
        self.0 |= ((flags.bits & 0x0F) as u64) << 52;
        self
    }

    fn set_access(&mut self, access: u8) -> &mut Descriptor {
        self.0 |= (access as u64) << 40;
        self
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Flags: u8 {
        const PAGE_GRANULARITY = 1 << 3;
        const PROTECTED_MODE = 1 << 2;
        const LONG_MODE = 1 << 1;
    }
}

bitflags! {
    // TODO: it may be 'better?' to have a separate struct for each type, considering the
    //       access flags formats differ between segment types
    #[derive(Default)]
    struct AccessFlags: u8 {
        const PRESENT = 1 << 7;
        const CODE_OR_DATA = 1 << 4;
        const EXECUTABLE = 1 << 3;
        const GROWS_DOWN = 1 << 2;
        const CONFORMING = 1 << 2;
        const READ_WRITE = 1 << 1;
    }
}

impl AccessFlags {
    #[allow(dead_code)]
    pub fn set_priviledge(mut self, ring: PriviledgeLevel) -> Self {
        // clears the current priviledge bits
        self.bits &= 0b1001_1111;

        // sets them to the passed in ring
        self.bits |= (ring as u8) << 5;

        self
    }
}
