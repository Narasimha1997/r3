#!/bin/bash

proj_root=$(pwd)
rm -r storage/tarfs
rm storage/tarfs.tar
mkdir -p storage/tarfs

# compile test assembly
pushd userland/test
    # compile assembly code
    nasm -f elf64 syscall.asm -o $proj_root/storage/tarfs/syscall.o
    ld -m elf_x86_64 -o $proj_root/storage/tarfs/syscall $proj_root/storage/tarfs/syscall.o
    rm $proj_root/storage/tarfs/syscall.o

    # compile C code
    gcc -m64 write.c -o $proj_root/storage/tarfs/write -nostdlib -ffreestanding -fomit-frame-pointer \
        -T ../configs/c_linker.ld -s -nostartfiles -z max-page-size=0x1000 -static
    gcc -m64 cpuid.c -o $proj_root/storage/tarfs/cpuid -nostdlib -ffreestanding -fomit-frame-pointer \
        -T ../configs/c_linker.ld -s -nostartfiles -z max-page-size=0x1000 -static

popd

pushd userland/userspace-rs
    cp target/x86_64/debug/echo_cli $proj_root/storage/tarfs/echo_cli
popd

# build tarfs
pushd storage/
    tar -cf tarfs.tar tarfs/
popd