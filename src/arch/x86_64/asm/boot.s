%define MB_MAGIC 0x36d76289     ; multiboot2 puts in eax 

%define PAGE_P  1 << 0          ; page present flag
%define PAGE_RW 1 << 1          ; page read/write flag
%define PAGE_H  1 << 7          ; page huge flag

%define PAE_FLAG  1 << 5        ; PAE flag in cr4

%define EFER_MSR 0xC0000080     ; EFER MSR register
%define EFER_LM_FLAG 1 << 8     ; EFER MSR LM flag

%define CR0_PG_FLAG 1 << 31     ; CR0 paging enabled flag

global _start

extern long_mode_start

section .text
bits 32
_start:
    ; setup stack
    mov esp, stack_top

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
    ; point first p4_table entry to p3_table
    mov eax, p3_table
    or eax, PAGE_RW | PAGE_P     ; set present and writable flags
    mov [p4_table + 0], eax      ; set the first entry of p4 to p3

    ; point first p3_table entry to p2_table
    mov eax, p2_table
    or eax, PAGE_RW | PAGE_P
    mov [p3_table + 0], eax

    ; map each p2 entry to a huge 2MiB page
    ; TODO: we need to use CPUID to check support, maybe should just
    ;       use normal pages instead
    mov ecx, 0                          ; for loop counter

    .map_p2_table_loop:
    mov eax, 0x200000
    mul ecx
    or eax, PAGE_H | PAGE_RW | PAGE_P   ; set page flags
    mov [p2_table + ecx * 8], eax       ; map the entry to our page

    inc ecx,                            ; for loop logic
    cmp ecx, 512
    jne .map_p2_table_loop

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

; apperently we rely on multiboot features? which ones?
; could be nice to put this in multiboot file, to separate
; concerns of this file away from anything multiboot related
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

section .rodata
; GDT for our jump to longmode
; Taken form osdev (which says it is from AMD programmers manual)
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

; create page tables and stack
section .bss
align 4096
p4_table:
    resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096
stack_bottom:
    resb 4096 * 4
stack_top:
