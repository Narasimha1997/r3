#include <stdint.h>

typedef uint64_t u64;
typedef int64_t i64;

#define SYSCALL_WRITE 1
#define SYSCALL_PID 9
#define SYSCALL_FORK 11
#define SYSCALL_EXECVP 59

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

int sys_execvp(u64 rdi) {
    u64 x;
    asm volatile(
        "int $0x80"
        : "=a"(x)
        : "0"(SYSCALL_EXECVP), "D"(rdi)
        : "rcx", "r11", "memory"
    );

    return x;
}

void _start()
{
    const char* buffer = "/sbin/syscall";
    sys_execvp((u64)buffer);
}
