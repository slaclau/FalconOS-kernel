ENTRY(start)

SECTIONS {
  . = 1M;

  .rodata :
  {
    /* ensure that the multiboot header is at the beginning */
    KEEP(*(.multiboot_header))
    *(.rodata .rodata.*)
    . = ALIGN(4K);
  }

  .text :
  {
    *(.text .text.*)
    . = ALIGN(4K);
  }

  .data :
  {
    *(.data .data.*)
    . = ALIGN(4K);
    _ring_buffer_start = ABSOLUTE(.);
    _ring_buffer_end = ABSOLUTE(. + 1024 * 128);
  }

  .bss :
  {
    *(.bss .bss.*)
    . = ALIGN(4K);
  }

  .got :
  {
    *(.got)
    . = ALIGN(4K);
  }

  .got.plt :
  {
    *(.got.plt)
    . = ALIGN(4K);
  }

  .data.rel.ro : ALIGN(4K) {
    *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    . = ALIGN(4K);
  }

  .gcc_except_table : ALIGN(4K) {
    *(.gcc_except_table)
    . = ALIGN(4K);
  }

  .eh_frame :
  {
    *(.eh_frame .eh_frame.*)
    . = ALIGN(4K);
  }
}