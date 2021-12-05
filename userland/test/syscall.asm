[bits 64]

global _start
section .text
_start:
LOOP_START:
  mov rax, 0x01
  int 0x80
  jmp LOOP_START
