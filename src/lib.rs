#![cfg_attr(not(test), no_std)]
#![feature(lang_items)]
#![feature(custom_test_frameworks)]
#![cfg_attr(test, allow(unused_imports))]

#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate volatile;

mod panic;
mod print;
mod vga;

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    loop {}
}
