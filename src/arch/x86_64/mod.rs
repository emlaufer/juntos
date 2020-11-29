pub mod gdt;
pub mod instructions;
pub mod interrupt;
pub mod paging;

use crate::BootInfo;
use gdt::GDT;
use interrupt::IDT;
use paging::{Page, VirtualAddress, PAGE_SIZE, PAGE_TABLE};

// TODO: This may be best moved to a more central locations
#[allow(dead_code)]
#[repr(u8)]
pub enum PriviledgeLevel {
    RingZero = 0,
    RingOne = 1,
    RingTwo = 2,
    RingThree = 3,
}

pub fn arch_init(stack_info: &BootInfo) {
    unsafe { GDT.load() };
    unsafe { IDT.load() };

    // set up a guard page at then end of the stack
    let mut pt = PAGE_TABLE.lock();
    // TODO: refactor this probably.
    // Need to add PAGE_SIZE cause guard page is NOT close enough
    let guard_page = Page::containing(VirtualAddress::from(
        stack_info.stack_top as usize + PAGE_SIZE,
    ));

    pt.modify(|mut mapper| {
        // TODO: handle possible errors?
        // This should not error though
        mapper.unmap(guard_page).expect("Issue mapping guard page");
    });
}
