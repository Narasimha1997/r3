#!/bin/bash

# build the kernel elf64 binary
pushd r3_kernel
    cargo xbuild --target x86_64.json
popd
