pub mod instructions;
pub mod interrupt;

use interrupt::IDT;

pub fn arch_init() {
    unsafe { IDT.load() };
}
