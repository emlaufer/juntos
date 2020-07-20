#![cfg_attr(not(test), no_std)]
#![cfg_attr(test, allow(unused_imports))]
#![feature(lang_items)]
#![feature(custom_test_frameworks)]
#![feature(asm)]

#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate volatile;

mod bochs;
mod panic;
mod print;
mod vga;

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("Hello, World!");
    magic_breakpoint!();
    loop {}
}
