[bits 64]

section .data
msg: db "Hello, World!", 10

global _start
section .text
_start:
LOOP_START:
  mov rax, 1
  mov rdi, 1
  mov rsi, msg
  mov rdx, 14
  int 0x80
  mov rax, 42
  int 0x80
  jmp LOOP_START

