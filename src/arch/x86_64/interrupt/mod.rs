mod handler;
pub mod idt;

use lazy_static::lazy_static;

use super::gdt::DOUBLE_FAULT_STACK_INDEX;
use crate::println;
use handler::exception;
use idt::{Descriptor, Idt};

lazy_static! {
    pub static ref IDT: Idt = {
        println!("Making idt...");
        let mut idt = Idt::new();

        // TODO: finish filling up whole IDT with handlers
        idt.div_by_zero = Descriptor::interrupt(exception::div_by_zero);
        idt.breakpoint = Descriptor::interrupt(exception::breakpoint);
        idt.invalid_opcode = Descriptor::interrupt(exception::invalid_opcode);

        // set double fault to use an IST
        idt.double_fault = {
            let mut desc = Descriptor::interrupt(exception::double_fault);
            desc.set_ist(DOUBLE_FAULT_STACK_INDEX as u8);
            desc
        };

        idt.segment_not_present = Descriptor::interrupt(exception::segment_not_present);
        idt.stack_segment_fault = Descriptor::interrupt(exception::stack_segment_fault);
        idt.general_protection_fault = Descriptor::interrupt(exception::general_protection_fault);
        idt.page_fault = Descriptor::interrupt(exception::page_fault);

        idt
    };
}
