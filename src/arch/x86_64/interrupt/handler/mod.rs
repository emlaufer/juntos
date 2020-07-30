/// This module is for x86 exception handling without using too much magic like the 'x86-interrupt'
/// feature.
pub mod exception;

// TODO: I am not sure a trait is the best way to represent this type of behavior, but I cannot
//       think of any other ways to do this while maintaining type checking and being generic.
/// A trait representing an interrupt handler, where the handler can be retrieves the function
/// pointer to the handler function. This way, we can still have type checking on Interrupt
/// Handlers without making unnecessary restrictions on the handler arguments or calling convention
/// (i.e. for system calls).
///
/// TODO: Is this the best way to do this, or should I just use a transparent struct as below?
///
/// ## Safety
///
/// This trait is unsafe because the implementation must ensure the returned function follows the
/// x86_64 calling convetion, which includes saving all registers on the stack, aligning
/// `esp` to a 16-byte boundary, restoring all registers off the stack, restoring `esp`,
/// and returning using the `iretq` instruction. The only exception to this is if there is another
/// calling convetion contract for software interrupts (i.e. with system calls which uses registers
/// to pass and return arguments).
pub unsafe trait InterruptHandler {
    fn raw_handler(&self) -> unsafe extern "C" fn() -> !;
}

// TODO: Do we want to make a separate one for exceptions with
//       error codes?
/// A transparent wrapper around an interrupt handler that is used
/// to typecheck interrupts. This way, you shouldn't be able to pass
/// a random `unsafe extern "C" fn() -> !` as an interrupt handler.
///
/// This should only be constructed by the `interrupt` macro, which
/// ensures the calling convention is correct. We guaruntee it cannot be
/// constructed outside this module because its field is private.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct StandardHandler(unsafe extern "C" fn() -> !);

// SAFETY: This is safe, as any StandardHandler must be created by the interrupt! macro within
//         this module, which ensures the calling convention is correct.
unsafe impl InterruptHandler for StandardHandler {
    fn raw_handler(&self) -> unsafe extern "C" fn() -> ! {
        self.0
    }
}

/// A transparent wrapper around an interrupt handler that is used
/// to typecheck interrupts. This way, you shouldn't be able to pass
/// a random `unsafe extern "C" fn() -> !` as an interrupt handler.
/// This type also expects an error code to be pushed onto the stack.
///
/// This should only be constructed by the `interrupt_error` macro, which
/// ensures the calling convention is correct. We guaruntee it cannot be
/// constructed outside this module because its field is private.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct HandlerWithError(unsafe extern "C" fn() -> !);

// SAFETY: This is safe, as any InterruptHandler must be created by the interrupt! macro within
//         this module, which ensures the calling convention is correct.
unsafe impl InterruptHandler for HandlerWithError {
    fn raw_handler(&self) -> unsafe extern "C" fn() -> ! {
        self.0
    }
}

/// A stack frame used for an interrupt with no error code.
#[derive(Debug)]
#[repr(C)]
pub struct InterruptStackFrame {
    instruction_pointer: usize,
    code_segment: usize,
    flags: usize,
    stack_pointer: usize,
    stack_segment: usize,
}

#[macro_export]
macro_rules! save_scratch_registers {
    () => {
        asm!(
            "push rax
             push rcx
             push rdx
             push rsi
             push rdi
             push r8
             push r9
             push r10
             push r11"
        );
    };
}

#[macro_export]
macro_rules! restore_scratch_registers {
    () => {
        asm!(
            "pop r11
             pop r10
             pop r9
             pop r8
             pop rdi
             pop rsi
             pop rdx
             pop rcx
             pop rax"
        );
    };
}

/// Defines an interrupt function
/// TODO
#[macro_export]
macro_rules! interrupt {
    ($handler:ident, |$stack_frame:ident| $code:block) => {
        // We cannot use concat_idents, due to concatenating function names
        paste::item! {
            #[allow(non_snake_case)]
            #[naked]
            pub unsafe extern "C" fn [<__raw_interrupt__ $handler>]() -> ! {
                extern "C" fn internal($stack_frame: &InterruptStackFrame) {
                    $code
                }

                // TODO: Technically, the rust book says:
                // "The requirement of restoring the stack pointer and non-output registers to
                // their original value only applies when exiting an asm! block."
                // I beleive we are breaking this, as we have multiple asm! blocks
                // next to each other that do not fix the stack pointer
                // Solution would be to combine this into a single asm! block

                save_scratch_registers!();

                asm!(
                    "mov rdi, rsp
                    add rdi, 9*8 
                    call {}",
                    in(reg) internal,
                    out("rdi") _
                );

                restore_scratch_registers!();

                asm!("iretq");

                ::core::intrinsics::unreachable();
            }
            // TODO: should I just use upper case name like a const? maybe not,
            //       considering this should be treated as a function really
            #[allow(non_upper_case_globals)]
            pub const $handler: StandardHandler = StandardHandler([<__raw_interrupt__ $handler>]);
        }
    };
}

// TODO: is there a way to reduce redundency between interrupt! and interrupt_error!?
/// Defines an interrupt function with an error code
/// TODO
#[macro_export]
macro_rules! interrupt_error {
    ($handler:ident, |$stack_frame:ident, $error_code:ident| $code:block) => {
        // We cannot use concat_idents, due to concatenating function names
        paste::item! {
            #[allow(non_snake_case)]
            #[naked]
            pub unsafe extern "C" fn [<__raw_interrupt__ $handler>]() -> ! {
                extern "C" fn internal($stack_frame: &InterruptStackFrame, $error_code: usize) {
                    $code
                }

                save_scratch_registers!();

                asm!(
                    "
                    mov rsi, [rsp + 9*8] // load error code
                    mov rdi, rsp
                    add rdi, 10*8 // load stack frame
                    sub rsp, 8 // align stack to 16 byte boundary
                    call {}
                    add rsp, 8 // undo stack alignment
                    ",
                    in(reg) internal,
                    out("rdi") _
                );

                restore_scratch_registers!();

                // return from interrupt handler
                asm!(
                    "
                    add rsp, 8 // pop error code off stack
                    iretq
                    "
                );
                ::core::intrinsics::unreachable();
            }
            // TODO: should I just use upper case name like a const? maybe not,
            //       considering this should be treated as a function really
            #[allow(non_upper_case_globals)]
            pub const $handler: HandlerWithError = HandlerWithError([<__raw_interrupt__ $handler>]);
        }
    };
}
