/// Includes Bochs debug helpers, specifically functions to insert 'magic breakpoints'

// TODO: Change this so it only inserts them on bochs builds
/// Inserts an inline 'magic_breakpoint' for the bochs debugger
#[macro_export]
macro_rules! magic_breakpoint {
    () => {
        // SAFETY: This should always be safe, as it does not change any state
        unsafe { asm!("xchg bx, bx") }
    };
}
