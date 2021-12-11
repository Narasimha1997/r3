#!/bin/bash

proj_root=$(pwd)
mkdir -p storage/tarfs
# compile test assembly
pushd userland/test
    # compile assembly code
    nasm -f elf64 syscall.asm -o $proj_root/storage/tarfs/syscall.o
    ld -m elf_x86_64 -o $proj_root/storage/tarfs/syscall $proj_root/storage/tarfs/syscall.o
    rm $proj_root/storage/tarfs/syscall.o

    # compile C code
    gcc -m64 write.c -o $proj_root/storage/tarfs/write -nostdlib -ffreestanding
popd

# build tarfs
pushd storage/
    tar -cf tarfs.tar tarfs/
popd