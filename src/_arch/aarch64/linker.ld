ENTRY(_start)

SECTIONS {
    . = 1M;

    .text :
    {
        *(.text._start) 
        *(.text*)
    }

    .rodata :
    {
        *(.rodata*)
    }

    .data :
    {
        *(.data*)
    }

     /* Section is zeroed in u64 chunks, align start and end to 8 bytes */
    .bss ALIGN(8):
    {
        __bss_start = .;
        *(.bss*);
        . = ALIGN(8);
        __bss_end = .;
    }
}