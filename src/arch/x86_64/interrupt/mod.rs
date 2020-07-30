mod handler;
pub mod idt;

use crate::println;
use handler::exception;
use idt::{Descriptor, Idt};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref IDT: Idt = {
        println!("Making idt...");
        let mut idt = Idt::new();

        // TODO: finish filling up whole IDT with handlers
        idt.div_by_zero = Descriptor::interrupt(exception::div_by_zero);
        idt.breakpoint = Descriptor::interrupt(exception::breakpoint);
        idt.invalid_opcode = Descriptor::interrupt(exception::invalid_opcode);
        idt.double_fault = Descriptor::interrupt(exception::double_fault);
        idt.segment_not_present = Descriptor::interrupt(exception::segment_not_present);
        idt.stack_segment_fault = Descriptor::interrupt(exception::stack_segment_fault);
        idt.general_protection_fault = Descriptor::interrupt(exception::general_protection_fault);
        idt.page_fault = Descriptor::interrupt(exception::page_fault);

        idt
    };
}
