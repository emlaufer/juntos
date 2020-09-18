OUTPUT_FORMAT(elf64-x86-64)

KERNEL_VOFFSET = 0xffff800000000000;

ENTRY(_start)

SECTIONS {
    . = 1M;

    /* boot code must remain down here due to booting to 32-bit */
    .boot :
    {
        KEEP(*(.multiboot_header))
    }

    .boot.data :
    {
       *(.boot.data) 
    }

    .boot.text :
    {
        *(.boot.text)
    }

    /* tell the linker to set the relocation addr to KERNEL_VOFFSET, but
     * actually place the sections immediatly after boot in the kernel.
     * i.e. code is actually at ~1M, but vaddrs written to higher half */
    . += KERNEL_VOFFSET;
    _kernel_start = .;
    .text ALIGN (4K) : AT (ADDR (.text) - KERNEL_VOFFSET)
    {
       *(.text .text.*)
    }

    .rodata ALIGN (4K) : AT (ADDR (.rodata) - KERNEL_VOFFSET)
    {
        *(.rodata .rodata.*)
    }

    .data.rel.ro ALIGN (4K) : AT (ADDR (.data.rel.ro) - KERNEL_VOFFSET)
    {
        *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    }

    .data ALIGN (4K) : AT (ADDR (.data) - KERNEL_VOFFSET)
    {
        *(.data .data.*)
    }

    .bss : AT (ADDR (.bss) - KERNEL_VOFFSET)
    {
        *(.bss .bss.*)

        /* setup page tables and stack */
        /* we export symbols for the bootloader to use
         * (because it wont have paging, we give it physical addresses */
        . = ALIGN (4K);
        p4_table = . - KERNEL_VOFFSET;
        . += 4K;
        p3_table = . - KERNEL_VOFFSET;
        . += 4K;
        p2_table = . - KERNEL_VOFFSET;
        . += 4K;
        p1_table = . - KERNEL_VOFFSET;
        . += 4K;
        stack_bottom = . - KERNEL_VOFFSET;
        . += 4 * 4K;
        stack_top = . - KERNEL_VOFFSET;

    }
    _kernel_end = .;
}
