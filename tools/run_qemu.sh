#!/bin/bash

QEMU_BINARY="qemu-system-x86_64"

if [[ "$1" == "--clean" || "$2" == "--clean" ]]; then
    rm -r kbin
fi

 mkdir -p kbin
    # build the kernel bootimage
./tools/sub/build_kernel_bootimage.sh

if [[ "$1" == "--build" || "$2" == "--build" ]]; then
    exit 0
fi

KERNEL_BIN_PATH="./kbin/x86_64/debug/bootimage-r3_kernel.bin"
# run in with qemu:
QEMU_ARGS="-enable-kvm -cpu host -m 1G -M pc --serial file:serial.out"
QEMU_ARGS="$QEMU_ARGS -hda $KERNEL_BIN_PATH"

# run the binary
$QEMU_BINARY $QEMU_ARGS
