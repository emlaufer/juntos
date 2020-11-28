/// Flushes the TLB
///
/// # Safety
/// Must be in kernel mode.
pub unsafe fn flush() {
    asm!(
        "mov rax, cr3
         mov cr3, rax",
         out("rax") _, // scratch
    );
}
