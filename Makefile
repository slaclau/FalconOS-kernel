arch ?= x86_64
kernel := build/kernel-$(arch).bin
iso := build/FalconOS-$(arch).iso
target ?= $(arch)-unknown-none
rust_os := target/$(target)/debug/libkernel.a

linker_script := crates/kernel/src/arch/$(arch)/loader/linker.ld
grub_cfg := grub.cfg
assembly_source_files := $(wildcard crates/kernel/src/arch/$(arch)/loader/*.asm)
assembly_object_files := $(patsubst crates/kernel/src/arch/$(arch)/loader%.asm, \
	build/arch/$(arch)/%.o, $(assembly_source_files))

.PHONY: all clean run iso kernel

all: $(kernel)

clean:
	@rm -r build

run: $(iso)
	@qemu-system-x86_64 -cdrom $(iso) -no-reboot -s

debug: $(iso)
	@qemu-system-x86_64 -cdrom $(iso) -no-reboot -no-shutdown -s -d int

gdb:
	@gdb $(kernel) -ex "target remote :1234"

iso: $(iso)

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub
	@grub2-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles

$(kernel): kernel $(rust_os) $(assembly_object_files) $(linker_script)
	@ld -n -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) $(rust_os)

kernel:
	@cargo build --target $(target)

build/arch/$(arch)/%.o: crates/kernel/src/arch/$(arch)/loader/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@
