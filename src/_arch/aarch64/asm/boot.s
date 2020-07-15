.section ".text._start"

.global _start

_start:
    bl  rust_main
1:  wfe
    b   1b
