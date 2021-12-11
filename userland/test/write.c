#include <stdint.h>

typedef uint64_t u64;
typedef int64_t i64;

#define SYSCALL_WRITE 1

i64 syscall_write(u64 fd, const void *buffer, u64 length)
{
    i64 ret_val;
    asm volatile(
        "int $0x80"
        : "=a"(ret_val)
        : "0"(SYSCALL_WRITE), "b"(fd), "c"(buffer), "d"(length)
        : "memory");

    return ret_val;
}

void _start()
{
    const char *buffer = "Hello, world!";
    while (1)
    {
        syscall_write(1, buffer, 14);
    }
}
