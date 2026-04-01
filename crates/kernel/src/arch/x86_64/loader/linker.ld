ENTRY(start)

KERNEL_START = 1M;
KERNEL_OFFSET = 0xFFFF800000000000;

SECTIONS {
  . = KERNEL_START;
  .loader :
  {
    /* ensure that the multiboot header is at the beginning */
    KEEP(*(.multiboot_header))
    *(.loader)
    . = ALIGN(4K);
  }

  . += KERNEL_OFFSET;
  .rodata : AT(ADDR(.rodata) - KERNEL_OFFSET)
  {
    *(.rodata .rodata.*)
    . = ALIGN(4K);
  }

  .text : AT(ADDR(.text) - KERNEL_OFFSET)
  {
    *(.text .text.*)
    . = ALIGN(4K);
  }

  .data : AT(ADDR(.data) - KERNEL_OFFSET)
  {
    *(.data .data.*)
    . = ALIGN(4K);
    //_ring_buffer_start = ABSOLUTE(.);
    //_ring_buffer_end = ABSOLUTE(. + 1024 * 128);
  }

  .bss : AT(ADDR(.bss) - KERNEL_OFFSET)
  {
    *(.bss .bss.*)
    . = ALIGN(4K);
  }

  .got : AT(ADDR(.got) - KERNEL_OFFSET)
  {
    *(.got)
    . = ALIGN(4K);
  }

  .got.plt : AT(ADDR(.got.plt) - KERNEL_OFFSET)
  {
    *(.got.plt)
    . = ALIGN(4K);
  }

  .data.rel.ro : AT(ADDR(.data.rel.ro) - KERNEL_OFFSET) {
    *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    . = ALIGN(4K);
  }

  .gcc_except_table : AT(ADDR(.gcc_except_table) - KERNEL_OFFSET) {
    *(.gcc_except_table)
    . = ALIGN(4K);
  }

  .eh_frame : AT(ADDR(.eh_frame) - KERNEL_OFFSET)
  {
    *(.eh_frame .eh_frame.*)
    . = ALIGN(4K);
  }
  _kernel_end = .;
}
