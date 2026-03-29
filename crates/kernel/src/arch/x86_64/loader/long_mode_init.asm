global long_mode_start

%define KERNEL_OFFSET 0xFFFF800000000000

section .bootstrap
bits 64
long_mode_start:
    ; load 0 into all data segment registers
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    extern p4_table
    mov rax, p4_table - KERNEL_OFFSET
    mov cr3, rax

    extern kernel_start
    add rsp, KERNEL_OFFSET
    mov rax, kernel_start
    call rax
    jmp rax

    ; print `OKAY` to screen
    mov rax, 0x2f592f412f4b2f4f
    mov qword [0xb8000], rax
    hlt