#![allow(dead_code)]
pub mod control;
pub mod segmentation;

#[macro_export]
macro_rules! define_read_reg_func {
    ($register:tt) => {
        pub fn $register() -> u16 {
            let seg: u16;
            unsafe { asm!(concat!("mov {:x}, ", stringify!($register)), out(reg) seg) };
            seg
        }
    };
}
