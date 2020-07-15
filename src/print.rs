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

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {print!("{}\n", format_args!($($arg)*))};
}
