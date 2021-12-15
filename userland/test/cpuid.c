#include <stdint.h>
typedef uint64_t u64;
typedef int64_t i64;

static const char *brand_string_heading = "Brand string is: ";
static const char* term = "/sbin/write";
static int a[10];

void syscall_write(u64 rdi, u64 rsi, u64 rdx)
{
    i64 ret_val;
    asm volatile(
        "int $0x80"
        : "=a"(ret_val)
        : "0"(1), "D"(rdi), "S"(rsi), "d"(rdx)
        : "rcx", "r11", "memory");
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

u64 find_length(char *string)
{
    char *temp_ptr = string;
    u64 length = 0;
    while (*temp_ptr != '\0')
    {
        length++;
        temp_ptr++;
    }

    return length;
}

void printf(const char *string, u64 length)
{
    syscall_write(1, (u64)string, length);
}

void brand_string(int eaxValues)
{
    if (eaxValues == 1)
    {
        __asm__("mov $0x80000002 , %eax\n\t");
    }
    else if (eaxValues == 2)
    {
        __asm__("mov $0x80000003 , %eax\n\t");
    }
    else if (eaxValues == 3)
    {
        __asm__("mov $0x80000004 , %eax\n\t");
    }
    __asm__("cpuid\n\t");
    __asm__("mov %%eax, %0\n\t"
            : "=r"(a[0]));
    __asm__("mov %%ebx, %0\n\t"
            : "=r"(a[1]));
    __asm__("mov %%ecx, %0\n\t"
            : "=r"(a[2]));
    __asm__("mov %%edx, %0\n\t"
            : "=r"(a[3]));
    
    u64 length = find_length((char*)&a[0]);
    printf((char *)&a[0], length);
}

void get_cpu_id()
{
    __asm__("xor %eax , %eax\n\t");
    __asm__("xor %ebx , %ebx\n\t");
    __asm__("xor %ecx , %ecx\n\t");
    __asm__("xor %edx , %edx\n\t");
    printf(brand_string_heading, 18);
    brand_string(1);
    brand_string(2);
    brand_string(3);
    printf("\n", 2);
}

void _start()
{
    u64 parent_pid = syscall_pid();
    u64 child_pid = syscall_fork();
    if (syscall_pid() == parent_pid) {
        get_cpu_id();
    } else {
        syscall_execv((u64)term);
    }

    while (1) {}
}