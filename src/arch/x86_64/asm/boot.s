%define MB_MAGIC 0x36d76289     ; multiboot2 puts in eax 

%define PAGE_P  1 << 0          ; page present flag
%define PAGE_RW 1 << 1          ; page read/write flag
%define PAGE_H  1 << 7          ; page huge flag

%define PAE_FLAG  1 << 5        ; PAE flag in cr4

%define EFER_MSR 0xC0000080     ; EFER MSR register
%define EFER_LM_FLAG 1 << 8     ; EFER MSR LM flag

%define CR0_PG_FLAG 1 << 31     ; CR0 paging enabled flag

global _start

; page table defined in linker script
extern p4_table
extern p3_table
extern p2_table
extern p1_table

; stack defined in linker script
extern stack_top
extern stack_bottom

; GDT for our jump to longmode
; Taken from osdev, though pretty self explanatory from AMD64 manual
section .boot.data
gdt64:
    .null: equ $ - gdt64         ; The null descriptor.
    dw 0xFFFF                    ; Limit (low).
    dw 0                         ; Base (low).
    db 0                         ; Base (middle)
    db 0                         ; Access.
    db 1                         ; Granularity.
    db 0                         ; Base (high).
    .code: equ $ - gdt64         ; The code descriptor.
    dw 0                         ; Limit (low).
    dw 0                         ; Base (low).
    db 0                         ; Base (middle)
    db 10011010b                 ; Access (exec/read).
    db 10101111b                 ; Granularity, 64 bits flag, limit19:16.
    db 0                         ; Base (high).
    .data: equ $ - gdt64         ; The data descriptor.
    dw 0                         ; Limit (low).
    dw 0                         ; Base (low).
    db 0                         ; Base (middle)
    db 10010010b                 ; Access (read/write).
    db 00000000b                 ; Granularity.
    db 0                         ; Base (high).
    .pointer:                    ; The GDT-pointer.
    dw $ - gdt64 - 1             ; Limit.
    dq gdt64                     ; Base.

section .boot.text
bits 32
_start:
    ; setup stack
    ; TODO: move the stack to somewhere else
    mov esp, stack_bottom

    ; put multiboot_info struct into rdi for rust calling convention
    ; TODO: probably better to move this, so we can use rdi if we want
    mov edi, ebx
    mov esi, eax

    call check_multiboot
    call check_cpuid
    call check_longmode

    call init_page_tables
    call enable_paging

    ; load the GDT to far jump to long mode
    lgdt [gdt64.pointer]
    jmp gdt64.code:long_mode_start

    hlt

; sets up a page table that identity maps the first GB of the kernel
; TODO: do we want to increase or decrease this?
init_page_tables:
    ; TODO: I am assuming I will do recursive mapping, but still not entirely
    ;       sure yet. Just in case, I wil set up a recursive mapping here
    mov eax, p4_table
    or eax, PAGE_RW | PAGE_P
    mov [p4_table + 511 * 8], eax

    ; point first p4_table entry to p3_table
    mov eax, p3_table
    or eax, PAGE_RW | PAGE_P     ; set present and writable flags
    mov [p4_table + 0], eax      ; set the first entry of p4 to p3
    mov [p4_table + 256 * 8], eax ; also map the pages to the higher half

    ; point first p3_table entry to p2_table
    mov eax, p2_table
    or eax, PAGE_RW | PAGE_P
    mov [p3_table + 0], eax

    ; point first p2_table entry to p2_table
    mov eax, p1_table
    or eax, PAGE_RW | PAGE_P
    mov [p2_table + 0], eax

    ; map each p1 entry to a 4K page
    mov ecx, 0                          ; for loop counter

    .map_p1_table_loop:
    mov eax, 0x1000
    mul ecx
    or eax, PAGE_RW | PAGE_P   ; set page flags
    mov [p1_table + ecx * 8], eax       ; map the entry to our page

    inc ecx,                            ; for loop logic
    cmp ecx, 512
    jne .map_p1_table_loop

    ret

; enables paging to page tables defined below
enable_paging:
    ; TODO check PAE support!!!
    ; enable PAE (big pages) by setting flag in cr4
    mov eax, cr4
    or eax, PAE_FLAG
    mov cr4, eax

    ; load p4_table to cr3 register
    mov eax, p4_table
    mov cr3, eax

    ; set the long mode bit in EFER 
    mov ecx, EFER_MSR
    rdmsr                               ; read the EFER msr
    or eax, EFER_LM_FLAG                ; set the long mode flag
    wrmsr

    ; enable paging in cr0 register
    mov eax, cr0
    or eax, CR0_PG_FLAG
    mov cr0, eax

    ret


; code below was taken from OSDev.org
check_multiboot:
    ; multiboot bootloader must write magic value to eax
    ; simply check it
    cmp eax, MB_MAGIC
    jne .no_multiboot
    ret

    .no_multiboot:
    mov al, "0"
    jmp error

check_longmode:
    ; check whether cpuid suports extended functions
    mov eax, 0x80000000    
    cpuid                  ; CPU identification.
    cmp eax, 0x80000001    ; Compare eax with 0x80000001.
    jb .no_longmode        ; It is less, there is no long mode.

    ; check for longmode
    mov eax, 0x80000001
    cpuid
    test edx, 1 << 29      ; Test if longmode bit is set in edx
    jz .no_longmode
    ret

    .no_longmode:
    mov al, "1"
    jmp error

check_cpuid:
    ; check CPUID support by attempting to flip the ID bit in the
    ; FLAGS register. If it can be flipped, CPUID available

    ; push flags into eax
    pushfd
    pop eax 

    ; move to ecx for comparing later
    mov ecx, eax

    ; flip id bit
    xor eax, 1 << 21

    ; copy eax to back to flags
    push eax
    popfd

    ; copy flags back to eax (with flipped bit if CPUID supported)
    pushfd
    pop eax

    ; restore flags back to original stored in ecx
    push ecx
    popfd

    ; compare eax and ecx. if they are equal, cpuid not supported
    xor eax, ecx
    jz .no_cpuid
    ret

    .no_cpuid:
    mov al, "2"
    jmp error

; from os.phil-opp.com
; prints ERR and given error code to screen
; parameter: err code (in ascii) in al
; TODO: better low level error handling with messages
error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt
; long mode entry point
bits 64
global long_mode_start
extern KERNEL_VOFFSET
extern kernel_main

extern vstack_bottom
extern vstack_top
extern _kernel_start
extern _kernel_end
extern _vkernel_start
extern _vkernel_end

; get the virtual higher-half addresses for the kernel stack
;vkstack_bottom: equ stack_bottom + KERNEL_VOFFSET
;vkstack_top: equ stack_top + KERNEL_VOFFSET

section .data
boot_info:
    dq vstack_bottom
    dq vstack_top
    dq _kernel_start
    dq _kernel_end
    dq _vkernel_start
    dq _vkernel_end


section .boot.text
long_mode_start:

    ; fix the stack pointer to point to higher half
    mov rax, KERNEL_VOFFSET
    add rax, rsp
    mov rsp, rax

    ; flush segment registers with null
    ; or they will have old values and iretq will cause a protection fault!
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; put boot info into third main param
    mov rdx, boot_info 

    ; must place address in register for near-aboslute call
    mov rax, kernel_main
    call rax

    ; print `OKAY` to screen
    mov rax, 0x2f592f412f4b2f4f
    mov qword [0xb8000], rax
    hlt

