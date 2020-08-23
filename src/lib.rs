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
mod panic;
mod print;
mod vga;

#[allow(dead_code)]
mod multiboot;

use multiboot::Multiboot2Info;

#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info: &Multiboot2Info, magic: u32) -> ! {
    // Run architecture specific initialization code
    arch::arch_init();

    println!("-- kernel_main end --");
    loop {}
}
