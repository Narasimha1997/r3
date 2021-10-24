#!/bin/bash

# install xbuild = this will make cross-compilation easy for us.
cargo install cargo-xbuild

# add llvm-tools-preview - the bootloader needs it.
rustup component add llvm-tools-preview 