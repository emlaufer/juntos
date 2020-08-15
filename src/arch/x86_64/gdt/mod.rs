mod gdt;
mod tss;

pub use gdt::SegmentSelector;

use gdt::Gdt;
use lazy_static::lazy_static;
use tss::Tss;

use super::instructions::registers::segmentation::*;
use crate::println;

pub const DOUBLE_FAULT_STACK_INDEX: u8 = 1;
const INTERRUPT_STACK_SIZE: usize = 4096;

// a stack for interrupts
// TODO: replace with memory allocator
static mut INTERRUPT_STACK: [u8; INTERRUPT_STACK_SIZE] = [0; INTERRUPT_STACK_SIZE];

lazy_static! {
    pub static ref TSS: Tss = {
        println!("Making tss...");
        let mut tss = Tss::new();

        // set the first index of the TSS IST to a new stack
        unsafe {
            let stack_addr = (&INTERRUPT_STACK as *const _) as u64;
            tss.interrupt_stacks[DOUBLE_FAULT_STACK_INDEX as usize - 1] = stack_addr + INTERRUPT_STACK_SIZE as u64;
        }

        tss
    };

    pub static ref GDT: Gdt = {
        println!("Making gdt...");

        let mut gdt = Gdt::new();

        // fill with normal 'dummy' segments, along with new tss
        let code_segment = gdt.add_code_segment(0, 0xFF0000);
        let data_segment = gdt.add_data_segment(0, 0);
        let tss_segment = gdt.add_tss(&TSS);

        // load the new gdt and flush the segments
        // TODO: we may want to move the loading to outside this ctor
        // SAFETY: We know this will be safe, as we just created the valid GDT, and are loading
        //         those segments. Of course, this depends on the correctness of the Gdt struct.
        unsafe {
            gdt.load();

            set_ds(data_segment);
            set_ds(data_segment);
            set_ss(data_segment);
            set_es(data_segment);
            set_fs(data_segment);
            set_gs(data_segment);
            set_cs(code_segment);
            load_tss(tss_segment);
        }

        gdt
    };
}

