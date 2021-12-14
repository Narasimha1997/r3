#include <stdint.h>

typedef uint64_t u64;
typedef int64_t i64;

i64 syscall(u64 rax, u64 rdi, u64 rsi, u64 rdx) {
    i64 ret_val;
    asm volatile(
        "int $0x80"
        : "=a"(ret_val)
        : "0"(rax), "D"(rdi), "S"(rsi), "d"(rdx)
        : "rcx", "r11", "memory");
    return ret_val;
}

void _start() {
    char buffer[4096];
    const char *welcome = "Welcome to ECHO program, I will echo whatever you say noob!.\n";
    const char *bullets = ">>>";
    i64 read_length = 0, iter = 0;
    syscall(1, 1, (u64)welcome, 62);
    while (1) {
        syscall(1, 1, (u64)bullets, 6);
        read_length = syscall(0, 0, (u64)buffer, 4096);
        syscall(1, 1, (u64)buffer, (u64)read_length);
        for (iter = 0; iter < read_length; iter++) {
            buffer[iter] = 0;
        }
    }
}
