#!/bin/bash

# install qemu if not exist:
function install_qemu () {
    if ! command -v qemu-system-x86_64 &> /dev/null; then
        # change this according to your package name
        echo "Installing Qemu + KVM"
        sudo apt update
        sudo apt install -y qemu-kvm libvirt-dev bridge-utils libvirt-daemon-system \
            libvirt-daemon virtinst bridge-utils libosinfo-bin libguestfs-tools \
            virt-top
        echo "Installed Qemu + KVM"
    else
        echo "Qemu with kvm is already installed"
    fi

    if ! command -v jq &> /dev/null; then
        sudo apt install jq
    fi
}

function install_rust_cargo () {
    if ! command -v cargo &> /dev/null; then
        echo "Installing rust and cargo."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
        echo "Installed rust and cargo."
    else
        echo "Rust and Cargo are already installed."
    fi
}

function setup_toolchain_env () {
    echo "Configuring project to use rust nightly cross compiler."
    ./tools/sub/get_cargo_prerequsites.sh

    echo "Building bootimage generator binary locally."
    ./tools/sub/build_bootimage_binary.sh

    echo "Done configuring."
}

function configure_vscode () {
    echo "Configuring VS code RLS plugin to use custom target."

    if [[ -d ".vscode" ]]; then
        rm -r .vscode
    fi

    if [[ -d "r3_kernel/.vscode" ]]; then
        rm -r r3_kernel/.vscode
    fi

    mkdir -p .vscode
    mkdir -p r3_kernel/.vscode

    targetpath=$PWD/r3_kernel/x86_64.json_text
    sysrootpath=$PWD/r3_kernel/target/sysroot
    deppath="-L dependency=$sysrootpath/lib/rustlib/x86_64-unknown-linux-gnu/lib"

    json_text='{
        "rust.target": $targetpath,
        "rust.all_targets": false,
        "rust.sysroot": $sysrootpath,
        "rust.rustflags": $deppath
    }'

    jq -n --arg targetpath "$targetpath" --arg sysrootpath "$sysrootpath" --arg deppath "$deppath" "$json_text" >> .vscode/settings.json
    jq -n --arg targetpath "$targetpath" --arg sysrootpath "$sysrootpath" --arg deppath "$deppath" "$json_text" >> r3_kernel/.vscode/settings.json
}

install_qemu
install_rust_cargo
setup_toolchain_env
configure_vscode