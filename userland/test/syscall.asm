[bits 64]

section .data
msg: db "Hello, World!", 10
t_sec: dq 4
t_usec: dq 0

global _start

_fork:
mov rax, 11
int 0x80
ret

section .text
_start:
call _fork
call _fork
call _fork
LOOP_START:
  mov rax, 1
  mov rdi, 1
  mov rsi, msg
  mov rdx, 14
  int 0x80
  mov rdi, t_sec
  mov rax, 46
  int 0x80
  jmp LOOP_START

