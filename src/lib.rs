#![cfg_attr(not(test), no_std)]
#![cfg_attr(test, allow(unused_imports))]
#![feature(lang_items)]
#![feature(custom_test_frameworks)]
#![feature(asm)]
#![feature(llvm_asm)]
#![feature(core_intrinsics)]
#![feature(naked_functions)]
#![feature(concat_idents)]
#![feature(linkage)]
#![feature(ptr_internals)]
#![feature(alloc_error_handler)]

//#[cfg(not(test))]
extern crate alloc;
#[cfg(not(test))]
mod kalloc;

mod arch;

mod bochs;
mod memory;
mod panic;
mod print;
mod utils;
mod vga;

#[allow(dead_code)]
mod multiboot;

use core::mem;
use multiboot::Multiboot2Info;

const MAGIC: u32 = 0x36d76289;

// just used to pass stack addresses from the bootloader
// into rust. Not sure if I really it
#[repr(C)]
pub struct BootInfo {
    // bottom as in where the stack starts (i.e. HIGH memory)
    stack_bottom: u64,
    stack_top: u64,
    pkernel_start: u64,
    pkernel_end: u64,
    vkernel_start: u64,
    vkernel_end: u64,
}

#[no_mangle]
pub extern "C" fn kernel_main(
    multiboot_info: &Multiboot2Info,
    magic: u32,
    boot_info: &BootInfo,
) -> ! {
    // ensure multiboot2 magic is correct (or else we were loaded by the wrong bootloader)
    assert!(magic == MAGIC);
    println!(
        "Stack bottom: {:x?} stack top: {:x?}",
        boot_info.stack_bottom, boot_info.stack_top
    );

    // TODO:
    // 1. remove mapping to lower half
    // 2. Add section in bss for starter bitmap

    // Run architecture specific initialization code
    arch::arch_init(boot_info);

    let multiboot_range = multiboot_info.memory_region();

    // TODO: this wont work due to higher half mapping. Just get it from linker instead
    //let _kernel_range = multiboot_info.elf_symbols().unwrap().kernel_memory_region();

    // subtract the memory regions for the kernel and multiboot header
    // Honestly should just make a data structure that manages this for me
    // TODO: should also copy it to kernel memory so I don't need to keep the multiboot struct
    //        around
    //

    let mmap = multiboot_info.memory_map().unwrap().available();
    let kernel_entry = mmap
        .filter(|entry| {
            entry.start_addr() <= boot_info.pkernel_start
                && entry.end_addr() >= boot_info.pkernel_end
        })
        .next()
        .expect("Couldn't find kernel in memory map!");
    println!("{:?}", multiboot_range);
    println!("{:?} {:?}", boot_info.pkernel_start, boot_info.pkernel_end);
    let mut main_region = PhysicalMemoryRegion::from_multiboot(kernel_entry);
    let kernel_region =
        main_region.take((boot_info.pkernel_end - main_region.base.as_u64()) as usize);

    println!("{:x?} {:x?}", kernel_region, main_region);
    println!(
        "{:x?} {:x?}",
        boot_info.vkernel_start, boot_info.vkernel_end
    );

    println!(
        "Stack size: {:x?} {:x?}",
        boot_info.stack_bottom, boot_info.stack_top
    );

    // TODO: if we don't save multiboot_region, we need to drop it
    mem::drop(multiboot_info);

    // TEST New alloc design
    use crate::memory::BootstrapAllocator;
    use crate::memory::{FrameAllocator, PhysicalMemoryRegion};
    unsafe { BootstrapAllocator::init(main_region) }
    let alloc = BootstrapAllocator::get();

    // TEST: check paging code
    use arch::x86_64::paging::{Page, VirtualAddress, PAGE_TABLE};
    use memory::Frame;
    use vga::{Color, ColorCode, VgaChar};
    let mut pte = PAGE_TABLE.lock();

    // try out the page table mappings
    let page = Page::containing(VirtualAddress::new(0xFFFF_DEAD_BEEF_B000));
    let frame = Frame::<BootstrapAllocator>::containing((0xB_8000) as usize);
    pte.modify(|mut page_table| page_table.map(page, frame, alloc));

    // Just write some random chars. Should only see a red T if it worked
    unsafe {
        *(0xB_8040 as *mut VgaChar) = VgaChar::new(b'U', ColorCode::new(Color::Red, Color::Black));
        *(0xFFFF_DEAD_BEEF_B040 as *mut VgaChar) =
            VgaChar::new(b'T', ColorCode::new(Color::Red, Color::Black));
    }

    println!("-- kernel_main end --");
    loop {}
}
