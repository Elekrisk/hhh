

.PHONY: all, all-debug, run, debug, install

all:
	cd bootloader && cargo build --release
	cd kernel && cargo build --release
	dd if=/dev/zero of=disk.fat bs=1M count=100
	sudo mkfs.vfat disk.fat
	mmd -i disk.fat ::EFI
	mmd -i disk.fat ::EFI/BOOT
	mcopy -i disk.fat target/x86_64-unknown-uefi/release/bootloader.efi ::EFI/BOOT/BOOTX64.EFI
	mcopy -i disk.fat target/target/release/kernel ::kernel.elf

all-qemu:
	cd bootloader && HHH_MAX_SCREEN_SIZE=(1920,900) cargo build --release
	cd kernel && cargo build --release
	dd if=/dev/zero of=disk.fat bs=1M count=100
	sudo mkfs.vfat disk.fat
	mmd -i disk.fat ::EFI
	mmd -i disk.fat ::EFI/BOOT
	mcopy -i disk.fat target/x86_64-unknown-uefi/release/bootloader.efi ::EFI/BOOT/BOOTX64.EFI
	mcopy -i disk.fat target/target/release/kernel ::kernel.elf

all-debug:
	cd bootloader && cargo build --release --features wait_for_gdb
	cd kernel && cargo build --release
	dd if=/dev/zero of=disk.fat bs=1M count=100
	sudo mkfs.vfat disk.fat
	mmd -i disk.fat ::EFI
	mmd -i disk.fat ::EFI/BOOT
	mcopy -i disk.fat target/x86_64-unknown-uefi/release/bootloader.efi ::EFI/BOOT/BOOTX64.EFI
	mcopy -i disk.fat target/target/release/kernel ::kernel.elf

run: all-qemu
	qemu-system-x86_64 -bios bios.bin disk.fat -no-reboot

debug: all-debug
	qemu-system-x86_64 -bios bios.bin disk.fat -s
	
install: all
	cp target/x86_64-unknown-uefi/release/bootloader.efi /run/media/elekrisk/6D95-4DD4/EFI/BOOT/BOOTX64.EFI
	cp target/target/release/kernel /run/media/elekrisk/6D95-4DD4/kernel.elf