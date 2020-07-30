use super::super::instructions;
use super::handler::{HandlerWithError, InterruptHandler, StandardHandler};
use core::marker::PhantomData;
use core::mem::size_of;

#[repr(C, packed)]
pub struct Idt {
    pub div_by_zero: Descriptor<StandardHandler>,
    pub debug: Descriptor<StandardHandler>,
    pub non_maskable_interrupt: Descriptor<StandardHandler>,
    pub breakpoint: Descriptor<StandardHandler>,
    pub overflow: Descriptor<StandardHandler>,
    pub bound_range_exceeded: Descriptor<StandardHandler>,
    pub invalid_opcode: Descriptor<StandardHandler>,
    pub device_not_available: Descriptor<StandardHandler>,
    pub double_fault: Descriptor<HandlerWithError>,
    pub coprocessor_segment_overrun: Descriptor<StandardHandler>,
    pub invalid_tss: Descriptor<HandlerWithError>,
    pub segment_not_present: Descriptor<HandlerWithError>,
    pub stack_segment_fault: Descriptor<HandlerWithError>,
    pub general_protection_fault: Descriptor<HandlerWithError>,
    pub page_fault: Descriptor<HandlerWithError>,

    /// Entry 15 is reserved
    _reserved1: Descriptor<StandardHandler>,

    pub floating_point_exception: Descriptor<StandardHandler>,
    pub alignment_check: Descriptor<HandlerWithError>,
    pub machine_check: Descriptor<StandardHandler>,
    pub simd_floating_point_exception: Descriptor<StandardHandler>,
    pub virtualization_exception: Descriptor<StandardHandler>,

    /// Entries 21-29 are reserved
    _reserved2: [Descriptor<StandardHandler>; 9],

    pub security_exception: Descriptor<HandlerWithError>,

    /// Entry 31 is reserved
    _reserved3: Descriptor<StandardHandler>,

    // TODO: These will need to change if we use something other than a StandardHandler for
    //       software interrupts
    /// Remaining descriptors for user-defined interrupts
    pub descriptors: [Descriptor<StandardHandler>; 256 - 32],
}

impl Idt {
    pub fn new() -> Self {
        Self {
            div_by_zero: Descriptor::new(),
            debug: Descriptor::new(),
            non_maskable_interrupt: Descriptor::new(),
            breakpoint: Descriptor::new(),
            overflow: Descriptor::new(),
            bound_range_exceeded: Descriptor::new(),
            invalid_opcode: Descriptor::new(),
            device_not_available: Descriptor::new(),
            double_fault: Descriptor::new(),
            coprocessor_segment_overrun: Descriptor::new(),
            invalid_tss: Descriptor::new(),
            segment_not_present: Descriptor::new(),
            stack_segment_fault: Descriptor::new(),
            general_protection_fault: Descriptor::new(),
            page_fault: Descriptor::new(),
            _reserved1: Descriptor::new(),
            floating_point_exception: Descriptor::new(),
            alignment_check: Descriptor::new(),
            machine_check: Descriptor::new(),
            simd_floating_point_exception: Descriptor::new(),
            virtualization_exception: Descriptor::new(),
            _reserved2: [Descriptor::new(); 9],
            security_exception: Descriptor::new(),
            _reserved3: Descriptor::new(),
            descriptors: [Descriptor::new(); 256 - 32],
        }
    }

    /// ## Safety: The caller must ensure that `self` is valid, and that it will continue to live
    ///            as long as it is needed (i.e. it may not live on the stack).
    pub unsafe fn load(&self) {
        let ptr = IdtPseudoDescriptor {
            base: self as *const _ as u64,
            limit: (size_of::<Self>() - 1) as u16,
        };

        asm!("lidt [{}]", in(reg) &ptr)
    }
}

/// Represents a pseudo-descriptor to an IDT, that is used in the lidt instruction
#[repr(C, packed)]
struct IdtPseudoDescriptor {
    limit: u16,
    base: u64,
}

// TODO: This may be best moved to a more central locations
#[allow(dead_code)]
#[repr(u8)]
pub enum PriviledgeLevel {
    RingZero = 0,
    RingOne = 1,
    RingTwo = 2,
    RingThree = 3,
}

#[allow(dead_code)]
#[repr(u8)]
pub enum DescriptorType {
    Interrupt = 0xE,
    Trap = 0xF,
}

// BitFlags was not working correctly for some reason?
// Perhaps it cannot handle multi-bit 'flags', i.e. 0xE, so lets define a different struct.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct DescriptorFlags(u8);

impl DescriptorFlags {
    pub fn new() -> DescriptorFlags {
        DescriptorFlags(0)
    }

    pub fn set_priviledge(mut self, ring: PriviledgeLevel) -> Self {
        // TODO: clearing may be uneccesary if we only set them once
        //       but the time wasted is probably minimal so it may be good to keep just in case

        // clears the current priviledge bits
        self.0 &= 0b1001_1111;

        // sets them to the passed in ring
        self.0 |= (ring as u8) << 5;

        self
    }

    pub fn set_type(mut self, desc_type: DescriptorType) -> Self {
        // clears the current type bits
        self.0 &= 0b1111_0000;

        // sets them to the passed in ring
        self.0 |= desc_type as u8;

        self
    }

    pub fn set_present(mut self) -> Self {
        // set present bit
        self.0 |= 1 << 7;

        self
    }
}

/// Represents an entry into the Interrupt Descriptor Table.
/// The format is specified as:
/// ```
///    0                   1                   2                   3
///    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |                           Reserved                            |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |                      Target Offset[63:31]                     |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |     Target Offset[31:16]      |       Type      |   Zero  |IST|
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |        Target Selector        |      Target Offset[15:0]      |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Descriptor<T: InterruptHandler> {
    /// Bits 0-15 of the target offset
    offset_low: u16,

    /// Selector in the GDT table of the interrupt function
    gdt_selector: u16,

    // TODO: Create struct for this if I ever need an interrupt stack
    /// Bits 0-2 hold the interrupt stack table offset, others 0
    ist: u8,

    /// IDT flags:
    /// ```
    ///   7                           0
    /// +---+---+---+---+---+---+---+---+
    /// | P |  DPL  | 0 |    GateType   |
    /// +---+---+---+---+---+---+---+---+
    /// ```
    /// Where:
    /// - P: present bit, 0 for unused entry
    /// - DPL: Descriptor Priviledge Level.
    /// - GateType: Type of the gate (i.e. trap, interrupt, exception)
    pub flags: DescriptorFlags,

    /// Bits 16-31 of the target offset
    offset_mid: u16,

    /// Bits 31-63 of the target offset
    offset_high: u32,

    /// Always 0
    _zero: u32,

    _phantom: PhantomData<T>,
}

impl<T: InterruptHandler> Descriptor<T> {
    pub fn new() -> Self {
        Self {
            offset_low: 0,
            gdt_selector: 0,
            ist: 0,
            flags: DescriptorFlags::new(),
            offset_mid: 0,
            offset_high: 0,
            _zero: 0,
            _phantom: PhantomData,
        }
    }

    pub fn interrupt(handler: T) -> Self {
        let descriptor = DescriptorFlags::new()
            .set_type(DescriptorType::Interrupt)
            .set_priviledge(PriviledgeLevel::RingOne)
            .set_present();

        let mut entry = Self::new();

        entry.set_offset(
            instructions::registers::segmentation::cs(),
            handler.raw_handler() as usize,
        );
        entry.set_flags(descriptor);
        entry
    }

    pub fn set_offset(&mut self, selector: u16, offset: usize) {
        self.gdt_selector = selector;
        self.offset_low = offset as u16;
        self.offset_mid = (offset >> 16) as u16;
        self.offset_high = (offset >> 32) as u32;
    }

    pub fn set_flags(&mut self, type_attr: DescriptorFlags) {
        self.flags = type_attr;
    }
}
