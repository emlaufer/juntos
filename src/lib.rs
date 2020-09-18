#![cfg_attr(not(test), no_std)]
#![cfg_attr(test, allow(unused_imports))]
#![feature(lang_items)]
#![feature(custom_test_frameworks)]
#![feature(asm)]
#![feature(llvm_asm)]
//#![feature(abi_x86_interrupt)] TODO: this may be better than naked functions
#![feature(core_intrinsics)]
#![feature(naked_functions)]
#![feature(concat_idents)]
#![feature(linkage)]

mod arch;

mod bochs;
mod memory;
mod panic;
mod print;
mod utils;
mod vga;

#[allow(dead_code)]
mod multiboot;

use memory::BumpAllocator;
use multiboot::Multiboot2Info;
use utils::IterExtras;

const MAGIC: u32 = 0x36d76289;

#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info: &Multiboot2Info, magic: u32) -> ! {
    // ensure multiboot2 magic is correct (or else we were loaded by the wrong bootloader)
    assert!(magic == MAGIC);

    // TODO: We are now higher half. What does this mean?
    // 1. we should make sure VGA writer still points to correct vaddr
    // 2. setup the idt, set up the gdt, set up page tables
    // 3. create a new kernel stack with a guard page
    // 4. remove mapping to lower half
    // 5. ALSO rewrite the spagetti for memory regions high key, or at least see if easier way?
    //      - Maybe soln is just to pass in the used regions, then it can just check on alloc!

    // Run architecture specific initialization code
    arch::arch_init();

    let multiboot_range = multiboot_info.memory_region();
    let kernel_range = multiboot_info.elf_symbols().unwrap().kernel_memory_region();

    // subtract the memory regions for the kernel and multiboot header
    let free_regions = multiboot_info
        .memory_map()
        .unwrap()
        .available_regions()
        .leftovers(|region| region.subtract(kernel_range))
        .leftovers(|region| region.subtract(multiboot_range));

    let _allocator = BumpAllocator::new(free_regions);

    println!("-- kernel_main end --");
    loop {}
}
