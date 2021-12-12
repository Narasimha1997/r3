#include <stdint.h>

typedef uint64_t u64;
typedef int64_t i64;

#define SYSCALL_WRITE 1
#define SYSCALL_PID 9
#define SYSCALL_FORK 11

i64 syscall_write(u64 rdi, u64 rsi, u64 rdx)
{
    i64 ret_val;
    asm volatile(
        "int $0x80"
        : "=a"(ret_val)
        : "0"(SYSCALL_WRITE), "D"(rdi), "S"(rsi), "d"(rdx)
        : "rcx", "r11", "memory"
    );

    return ret_val;
}

int syscall_fork()
{
    u64 x;
    asm volatile(
        "int $0x80"
        : "=a"(x)
        : "0"(SYSCALL_FORK)
    );

    return x;
}

void _start()
{
    // const char* buffer = "Hello, parent\n";
    // fork:
    syscall_fork();
    while (1);
}
