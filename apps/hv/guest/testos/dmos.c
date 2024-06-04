
// 单个字符输出函数
void uart_putchar(char c) {
    volatile unsigned int * const UART0DR = (unsigned int *)0xFEB50000;
    *UART0DR = (unsigned int)c;
}

// 字符串输出函数
void uart_putstr(const char *str) {
    while (*str) {
        uart_putchar(*str++);
    }
}

// main.c
void kernel_main(void) {
    // 在这里可以添加你的内核代码
    uart_putstr("hello world");
}
