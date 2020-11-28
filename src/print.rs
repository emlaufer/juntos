use core::fmt::{Arguments, Write};

use crate::vga::VGA_WRITER;

#[cfg(not(test))]
#[doc(hidden)]
pub fn _print(args: Arguments) {
    VGA_WRITER.lock().write_fmt(args).unwrap();
}

// Allows us to print in the kernel during testing.
// Very useful.
#[cfg(test)]
#[doc(hidden)]
pub fn _print(args: Arguments) {
    std::print!("{}", args)
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {$crate::print::_print(format_args!($($arg)*))};
}

// TODO: Make println not use print, because we have to import both which is annoying
#[macro_export]
macro_rules! println {
    () => {$crate::print::_print(format_args!("\n"))};
    ($($arg:tt)*) => {$crate::print::_print(format_args!("{}\n", format_args!($($arg)*)))};
}
