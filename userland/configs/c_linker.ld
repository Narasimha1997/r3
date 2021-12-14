ENTRY(_start)
VIRT_START = 0x400000;
SECTIONS
{
    . = VIRT_START;
    rodata : ALIGN(4k) {
        *(.rodata.*)
		*(.rodata)
    }
    .text : ALIGN(4k) {
        *(.text.*)
		*(.text)
    }
    .data : ALIGN(4k) { 
        *(.data.*)
		*(.data) 
    }
    .bss : ALIGN(4k) {
        *(.bss.*)
		*(.bss)
		*(COMMON)
    }
}