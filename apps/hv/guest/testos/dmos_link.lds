/* link.ld */
ENTRY(_start)

SECTIONS
{
    . = 0x43a000;  /* 设置内核加载地址 */

    .text : {
        *(.text)
    }

    .rodata : {
        *(.rodata)
    }

    .data : {
        *(.data)
    }

    .bss : {
        *(.bss)
    }
}
