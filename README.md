# Juntos

A simple Operating System I am writing to learn more about
OS implementation, the x86 architecture, and bare metal Rust.

Huge thanks to the following sources for providing the lessons and information
that I used to help write this:

- https://os.phil-opp.com/
- https://intermezzos.github.io/
- https://github.com/rust-embedded/rust-raspberrypi-OS-tutorials
- https://wiki.osdev.org/
- https://www.amd.com/system/files/TechDocs/24593.pdf

## Features

A basic list of features I want to implement:

- [x] Boots into the Rust kernel
- [x] Wrapper for the VGA text buffer
- [ ] Basic interrupt and exception handling
- [ ] Virtual Memory
- [ ] Scheduling
- [ ] Aarch64 support

Optional features that would be nice to add:

- [ ] Full build using build.rs and custom cargo commands instead of a Makefile
- [ ] Moving as much of the x86_64 bootloader from asm to Rust
