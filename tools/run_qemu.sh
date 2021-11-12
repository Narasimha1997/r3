#!/bin/bash

QEMU_BINARY="qemu-system-x86_64"

if [[ "$1" == "--clean" || "$2" == "--clean" || "$3" == "--clean" ]]; then
    rm -r kbin
fi

 mkdir -p kbin
    # build the kernel bootimage
./tools/sub/build_kernel_bootimage.sh

if [[ "$1" == "--build" || "$2" == "--build" || "$3" == "--build" ]]; then
    exit 0
fi


KERNEL_BIN_PATH="./kbin"
QEMU_ARGS="-enable-kvm -cpu host -m 1G -M pc --serial file:serial.out"

if [[ "$1" == "--uefi" || "$2" == "--uefi" || "$3" == "--uefi" ]]; then
    KERNEL_BIN_PATH="$KERNEL_BIN_PATH/boot-uefi-r3_kernel.img"
    QEMU_ARGS="$QEMU_ARGS -bios /usr/share/ovmf/OVMF.fd"
else
    KERNEL_BIN_PATH="$KERNEL_BIN_PATH/boot-bios-r3_kernel.img"
fi

# run in with qemu:

QEMU_ARGS="$QEMU_ARGS -hda $KERNEL_BIN_PATH"

# run the binary
$QEMU_BINARY $QEMU_ARGS
