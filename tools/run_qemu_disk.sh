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


STORAGE_DISK=storage/tarfs.tar
KERNEL_BIN_PATH="./kbin"
INET_1="-netdev tap,id=r3net,script=no,downscript=no -device rtl8139,netdev=r3net -object filter-dump,id=r3net,netdev=r3net,file=net_dump.dat"
INET_2="-nic tap,model=rtl8139"
INET_3="-netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=r3_net -device rtl8139,netdev=r3_net,id=r3_net -object filter-dump,id=r3_net,netdev=r3_net,file=net_dump.dat"


QEMU_ARGS="-enable-kvm -cpu host -m 1G -monitor stdio --serial file:serial.out -drive file=$STORAGE_DISK,format=raw,index=1,media=disk $INET_2"

if [[ "$1" == "--uefi" || "$2" == "--uefi" || "$3" == "--uefi" ]]; then
    KERNEL_BIN_PATH="$KERNEL_BIN_PATH/boot-uefi-r3_kernel.img"
    QEMU_ARGS="$QEMU_ARGS -bios /usr/share/ovmf/OVMF.fd"
else
    KERNEL_BIN_PATH="$KERNEL_BIN_PATH/boot-bios-r3_kernel.img"
fi

# run in with qemu:

QEMU_ARGS="$QEMU_ARGS -hda $KERNEL_BIN_PATH"

# run the binary
sudo $QEMU_BINARY $QEMU_ARGS
