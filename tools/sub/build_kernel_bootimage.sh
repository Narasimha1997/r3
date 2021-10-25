#!/bin/bash

# first build the kernel
pushd r3_kernel
    echo "Compiling kernel..."
    cargo xbuild
popd

KERNEL_PATH=$PWD/r3_kernel
KERNEL_KBIN_PATH=$PWD/kbin

# build the bootimage now:
pushd third_party/crates/bootloader
    echo "Building kernel bootimage..."
    cargo builder --kernel-manifest $KERNEL_PATH/Cargo.toml \
                  --kernel-binary $KERNEL_KBIN_PATH/x86_64/debug/r3_kernel \
                  --out-dir $KERNEL_KBIN_PATH
popd