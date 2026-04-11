arch ?= x86_64
kernel := build/kernel-$(arch).bin
iso := build/FalconOS-$(arch).iso
target ?= $(arch)-unknown-none
rust_os := target/$(target)/debug/libkernel.a

rust_bootstrap := target/$(target)/debug/bootstrap
bootstrap_tar := build/bootstrap.tar

linker_script := crates/kernel/src/arch/$(arch)/loader/linker.ld
grub_cfg := grub.cfg
assembly_source_files := $(wildcard crates/kernel/src/arch/$(arch)/loader/*.asm)
assembly_object_files := $(patsubst crates/kernel/src/arch/$(arch)/loader%.asm, \
	build/arch/$(arch)/%.o, $(assembly_source_files))

.PHONY: all clean run iso kernel

all: $(kernel)

clean:
	@rm -r build
	@rm -r target

run: $(iso)
	@qemu-system-x86_64 -cdrom $(iso) -no-reboot -s -debugcon stdio

debug: $(iso)
	@qemu-system-x86_64 -cdrom $(iso) -no-reboot -no-shutdown -s -d int

gdb:
	@gdb $(kernel) -ex "target remote :1234"

iso: $(iso)

$(iso): $(kernel) $(bootstrap_tar) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(bootstrap_tar) build/isofiles/boot/bootstrap.tar
	@cp $(grub_cfg) build/isofiles/boot/grub
	@grub2-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles

$(kernel): rust_code $(rust_os) $(assembly_object_files) $(linker_script)
	@ld -n -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) $(rust_os)

rust_code:
	@cargo build --target $(target) -p kernel
	RUSTFLAGS="-C link-arg=-no-pie" cargo build --target $(target) -p bootstrap

build/arch/$(arch)/%.o: crates/kernel/src/arch/$(arch)/loader/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@

$(bootstrap_tar): rust_code
	@mkdir -p build/tarfiles/bootstrap
	@cp $(rust_bootstrap) build/tarfiles/bootstrap/main
	@tar -c -f $(bootstrap_tar) -C build/tarfiles .
	@rm -r build/tarfiles
