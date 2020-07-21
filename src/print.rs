use crate::vga::VGA_WRITER;
use core::fmt::{Arguments, Write};

#[doc(hidden)]
pub fn _print(args: Arguments) {
    VGA_WRITER.lock().write_fmt(args).unwrap();
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
