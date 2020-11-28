#![allow(dead_code)]
pub mod control;
pub mod segmentation;

#[macro_export]
macro_rules! define_read_reg_func {
    ($register:tt, $width:tt) => {
        pub fn $register() -> $width {
            let seg: $width;
            unsafe { asm!(concat!("mov {:x}, ", stringify!($register)), out(reg) seg) };
            seg
        }
    };
}
