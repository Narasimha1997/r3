[bits 64]

section .data
msg: db "Hello, World!", 10

global _start
section .text
_start:
LOOP_START:
  mov rax, 1      ; syscall 1 - write
  mov rdi, 1      ; select stdout
  mov rsi, msg    ; pointer to message in .data
  mov rdx, 14     ; length
  int 0x80
  jmp LOOP_START
