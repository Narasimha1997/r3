#!/bin/bash

export PATH=$PATH:$PWD/third_party/builds/bin
pushd r3_kernel
    cargo bootimage
popd