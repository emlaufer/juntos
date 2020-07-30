%define MAGIC 0xe85250d6    ; multiboot 2 magic number
%define MODE 0              ; x86 32-bit protected mode

%define TYPE 0              ; we are not using any tags, so just provide terminating
%define FLAG 0
%define SIZE 8

section .multiboot_header
header_start:
    dd MAGIC
    dd MODE
    dd header_end - header_start ; header length

    ; checksum (when added to other feilds is 0)
    dd 0x100000000 - (MAGIC + 0 + (header_end - header_start))

    ; terminating tag
    dw TYPE
    dw FLAG
    dd SIZE
header_end: