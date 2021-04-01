#!/bin/bash

dd if=/dev/zero of=disk.fat bs=1M count=100
mkfs.vfat disk.fat
mmd -i disk.fat ::EFI
mmd -i disk.fat ::EFI/BOOT
mcopy -i disk.fat target/x86_64-unknown-uefi/release/bootloader.efi ::EFI/BOOT/BOOTX64.EFI
mcopy -i disk.fat target/target/release/kernel ::kernel.elf