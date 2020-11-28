use super::super::super::gdt::SegmentSelector;
use crate::define_read_reg_func;

/// A macro for defining a function to set the segment registers
macro_rules! define_set_seg_func {
    ($register:tt) => {
        paste::item! {
            pub unsafe fn [<set_$register>](segment: SegmentSelector) {
                asm!(
                    concat!("mov ", stringify!($register), ", ax"),
                        in("ax") segment.0
                );
            }
        }
    };
}

define_read_reg_func!(cs, u16);
define_read_reg_func!(ds, u16);
define_read_reg_func!(ss, u16);
define_read_reg_func!(es, u16);
define_read_reg_func!(fs, u16);
define_read_reg_func!(gs, u16);

define_set_seg_func!(ds);
define_set_seg_func!(ss);
define_set_seg_func!(es);
define_set_seg_func!(fs);
define_set_seg_func!(gs);

/// Function sets the CS register, by setting it
/// on a stack in a function, and returning.
/// We cannot (as far as I know) do
/// `jmp 0x8:tag` because inline assembly won't allow it
pub unsafe fn set_cs(segment: SegmentSelector) {
    // we have to do wacky stuff to set the cs register, as llvm wont let use use
    // jmp 0x8:1
    let inner = || {
        asm!(
            "
            push rdi
            lea rax, 1f[rip]
            push rax
            rex64 retf  // pops both rip and cs from stack!
            1:",
            in("rdi") segment.0, out("rax") _
        )
    };

    inner();
}

pub unsafe fn load_tss(segment: SegmentSelector) {
    asm!(
        "ltr ax",
        in("ax") segment.0
    );
}
