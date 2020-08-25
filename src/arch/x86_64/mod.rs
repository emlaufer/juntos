pub mod gdt;
pub mod instructions;
pub mod interrupt;
pub mod paging;

use gdt::GDT;
use interrupt::IDT;

// TODO: This may be best moved to a more central locations
#[allow(dead_code)]
#[repr(u8)]
pub enum PriviledgeLevel {
    RingZero = 0,
    RingOne = 1,
    RingTwo = 2,
    RingThree = 3,
}

pub fn arch_init() {
    unsafe { GDT.load() };
    unsafe { IDT.load() };
}
