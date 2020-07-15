global long_mode_start

extern rust_main

section .text
bits 64
long_mode_start:
    call rust_main

    ; print `OKAY` to screen
    mov rax, 0x2f592f412f4b2f4f
    mov qword [0xb8000], rax
    hlt