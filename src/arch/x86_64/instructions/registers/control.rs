use crate::define_read_reg_func;

define_read_reg_func!(cr0, u64);
define_read_reg_func!(cr2, u64);
define_read_reg_func!(cr3, u64);
define_read_reg_func!(cr4, u64);
