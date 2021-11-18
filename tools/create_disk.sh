#!/bin/bash

mkdir -p storage

# create a 100MB disk
qemu-img create storage/ATA_0.img 100M
