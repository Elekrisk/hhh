ENTRY(_start);

SECTIONS
{
    . = 0xFFFF800000000000;

    .text : {
        *(.text .text.*)
    }

    .data : {
        *(.data)
        *(.data.*)
    }

    .rodata : {
        *(.rodata)
        *(.rodata.*)
    }
    
    .bss : {
        *(.bss)
        *(.bss.*)
    }

    .got : {
        *(.got)
    }
}