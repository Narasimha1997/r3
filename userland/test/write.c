#include <stdint.h>

typedef uint64_t u64;
typedef int64_t i64;

static char buffer[4096];
static const char *welcome = "Welcome to ECHO program, I will echo whatever you say noob!.\n";
static const char *bullets = ">>> ";

// syscall 0 - read
// syscall 1 - write

typedef struct
{
    u64 seconds;
    u64 microseconds;
} __attribute__((packed)) timeval_t;

i64 syscall(u64 rax, u64 rdi, u64 rsi, u64 rdx)
{
    i64 ret_val;
    asm volatile(
        "int $0x80"
        : "=a"(ret_val)
        : "0"(rax), "D"(rdi), "S"(rsi), "d"(rdx)
        : "rcx", "r11", "memory");
    return ret_val;
}

i64 syscall_sleep(u64 rdi)
{
    i64 ret_val;
    asm volatile(
        "int $0x80"
        : "=a"(ret_val)
        : "0"(46), "D"(rdi)
        : "rcx", "r11", "memory");
    return ret_val;
}

<<<<<<< HEAD
i64 syscall_exit(u64 rdi)
{
    i64 ret_val;
    asm volatile(
        "int $0x80"
        : "=a"(ret_val)
        : "0"(4), "D"(rdi)
        : "rcx", "r11", "memory");
    return ret_val;
}

u64 syscall_fork()
{
    i64 ret_val;
    asm volatile(
        "int $0x80"
        : "=a"(ret_val)
        : "0"(11)
        : "rcx", "r11", "memory");
    return ret_val;
}

u64 syscall_pid()
{
    i64 ret_val;
    asm volatile(
        "int $0x80"
        : "=a"(ret_val)
        : "0"(9)
        : "rcx", "r11", "memory");
    return ret_val;
}

void syscall_execv(u64 rdi)
{
    i64 ret_val;
    asm volatile(
        "int $0x80"
        : "=a"(ret_val)
        : "0"(59), "D"(rdi)
        : "rcx", "r11", "memory");
}

void syscall_wait(u64 rdi)
{
    i64 ret_val;
    asm volatile(
        "int $0x80"
        : "=a"(ret_val)
        : "0"(47), "D"(rdi)
        : "rcx", "r11", "memory");
}

void syscall_shutdown()
{
    i64 ret_val;
    asm volatile(
        "int $0x80"
        : "=a"(ret_val)
        : "0"(48)
        : "rcx", "r11", "memory");
}

void exec_cpuid()
{
    u64 child = syscall_fork();

    if (syscall_pid() == child)
    {
        syscall_execv((u64)cpuid_term);
    }
    else
    {
        syscall_wait(child);
    }
}

=======
>>>>>>> 7814cbb7259dee237460843d6315ab1d1af066b1
void _start()
{
    i64 read_length = 0, iter = 0, n_times = 0;
    timeval_t sleep_time;
    sleep_time.seconds = 1;
    sleep_time.microseconds = 0;

    syscall(1, 1, (u64)welcome, 62);
    for (;;)
    {
        syscall(1, 1, (u64)bullets, 5);
        read_length = syscall(0, 0, (u64)buffer, 4096);
        syscall(1, 1, (u64)buffer, (u64)read_length);
        syscall_sleep((u64)(&sleep_time));
        for (iter = 0; iter < read_length; iter++)
        {
            buffer[iter] = 0;
        }
    }
<<<<<<< HEAD

    syscall_shutdown();
=======
>>>>>>> 7814cbb7259dee237460843d6315ab1d1af066b1
}
