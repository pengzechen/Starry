// startup.S
.section .text
.global _start

_start:
    // 设置栈指针
    ldr x0, =_stack_top
    mov sp, x0

    // 调用 C 语言的 main 函数
    bl kernel_main

    // 死循环，防止返回
1:  b 1b

// 栈空间
.section .bss
.align 12
.global _stack_top
_stack_top:
    .skip 0x2000  // 4KB 的栈空间
