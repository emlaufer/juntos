ARCH ?= x86_64

ifeq ($(ARCH), x86_64)
	AS := nasm
	ASFLAGS := -f elf64

	LD := ld

	QEMU := qemu-system-x86_64
	QEMU_MACHINE := pc-i440fx-5.0

endif

KERNEL_BIN := target/$(ARCH)/kernel-$(ARCH).bin
#KERNEL_BIN := target/$(ARCH)/debug/juntos
ISO := target/$(ARCH)/os-$(ARCH).iso

# TODO: Debug or release target?
KERNEL_LIB := target/$(ARCH)/debug/libjuntos.a

LDFLAGS := -n -b elf64-x86-64

ASMDIR := src/arch/$(ARCH)/asm
OBJDIR := target/$(ARCH)/obj/

ASMSRC := $(wildcard $(ASMDIR)/*.s)
ASMOBJ := $(patsubst $(ASMDIR)/%.s, $(OBJDIR)/%.o, $(ASMSRC))

LINK_SCRIPT := src/arch/$(ARCH)/linker.ld

.PHONY: all $(KERNEL_LIB) kernel iso qemu clean

all: $(KERNEL_BIN)

qemu: $(ISO)
	$(QEMU) -cdrom $(ISO) -machine $(QEMU_MACHINE) --enable-kvm

test:
	cargo test

clean:
	cargo clean
	rm bochslog.txt

kernel: $(KERNEL_BIN)

iso: $(ISO)

bochs: $(ISO)
	bochs -f bochs/bochs.$(ARCH) -q

objdump: $(KERNEL_BIN)
	objdump -D $(KERNEL_BIN)

$(KERNEL_LIB):
	RUST_TARGET_PATH=$(shell pwd)/src/arch/$(ARCH) cargo xbuild --target $(ARCH)

$(KERNEL_BIN): $(KERNEL_LIB) $(ASMOBJ) $(LINK_SCRIPT)
	echo $(ASMSRC)
	$(LD) $(LDFLAGS) -T $(LINK_SCRIPT) -o $(KERNEL_BIN) $(ASMOBJ) $(KERNEL_LIB)

$(ISO): $(KERNEL_BIN)
	mkdir -p target/$(ARCH)/isofiles/boot/grub
	cp $(KERNEL_BIN) target/$(ARCH)/isofiles/boot/kernel.bin
	cp grub/grub.cfg target/$(ARCH)/isofiles/boot/grub
	grub-mkrescue -o $(ISO) target/$(ARCH)/isofiles

$(OBJDIR)/%.o: $(ASMDIR)/%.s
	mkdir -p $(shell dirname $@)
	$(AS) $(ASFLAGS) -o $@ $<
