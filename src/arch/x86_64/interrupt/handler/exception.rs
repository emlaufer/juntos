use super::{HandlerWithError, InterruptStackFrame, StandardHandler};
use crate::println;
use crate::{interrupt, interrupt_error, restore_scratch_registers, save_scratch_registers};
use bitflags::bitflags;

bitflags! {
    struct PageFaultError: usize {
        const PROTECTION_VIOLATION = 1 << 0;
        const CAUSED_BY_WRITE = 1 << 1;
        const USER = 1 << 2;
        const RESERVED_WRITE = 1 << 3;
        const CAUSED_BY_INSTR_FETCH = 1 << 4;
    }
}

interrupt!(invalid_opcode, |stack_frame| {
    println!("OPCODE: not handled!");
    println!("{:x?}", stack_frame);
    loop {}
});

interrupt!(div_by_zero, |stack_frame| {
    println!("Exception: div by 0 SHOOK ULTIMATE");
    println!("{:x?}", stack_frame);
    loop {}
});

interrupt!(breakpoint, |stack_frame| {
    println!("Exception: BREAKPOINT");
    println!("{:x?}", stack_frame);
});

interrupt_error!(page_fault, |stack_frame, error_code| {
    let pagefault_error = PageFaultError::from_bits(error_code).unwrap();
    println!(
        "\nEXCEPTION: PAGE FAULT with error code {:?}\n{:#?}",
        pagefault_error, stack_frame
    );
    loop {}
});

interrupt_error!(segment_not_present, |stack_frame, error_code| {
    println!(
        "\nEXCEPTION: Segment not present {:?} code {:?}",
        stack_frame, error_code
    );
    loop {}
});

interrupt_error!(stack_segment_fault, |stack_frame, error_code| {
    println!(
        "\nEXCEPTION: stack segment fault {:?} code {:?}",
        stack_frame, error_code
    );
    loop {}
});

interrupt_error!(general_protection_fault, |stack_frame, error_code| {
    println!(
        "\nEXCEPTION: general protection fault {:x?} with code {:x}",
        stack_frame, error_code
    );
    loop {}
});

interrupt_error!(double_fault, |stack_frame, error_code| {
    println!(
        "\nDOUBLE FAULT: {:x?} with code {:x}",
        stack_frame, error_code
    );
    crate::magic_breakpoint!();

    // Double faults are not allowed to return.
    loop {}
});
