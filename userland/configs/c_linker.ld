ENTRY(_start)
VIRT_START = 0x400000;
SECTIONS
{
    . = VIRT_START;
    .text : ALIGN(4k) { *(.text) }
    .data : ALIGN(4k) { *(.data) }
    .bss : ALIGN(4k) { *(.bss) }
    .rodata : ALIGN(4k) { *(.rodata) }
}