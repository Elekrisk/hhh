
import subprocess
import sys
import platform
import json

class Options:
    def __init__(self):
        self.os = None
        self.debug = False

def build(options):
    cargo_command = ["cargo", "build"]
    if options.debug == False:
        cargo_command.append("--release")
    subprocess.run(cargo_command, cwd = "./kernel", check=True)
    if options.debug:
        cargo_command.append("--features=wait_for_gdb")
    subprocess.run(cargo_command, cwd = "./bootloader", check=True)
    pathpart = "debug" if options.debug else "release"
    commands = [
        "dd if=/dev/zero of=disk.fat bs=1M count=100",
        "mkfs.vfat disk.fat",
        "mmd -i disk.fat ::EFI",
        "mmd -i disk.fat ::EFI/BOOT",
        "mcopy -i disk.fat target/x86_64-unknown-uefi/"+pathpart+"/bootloader.efi ::EFI/BOOT/BOOTX64.EFI",
        "mcopy -i disk.fat target/target/"+pathpart+"/kernel ::kernel.elf"
    ]
    if options.os == "windows":
        for command in commands:
            subprocess.run("wsl -- " + command, check=True)
    else:
        for command in commands:
            subprocess.run(command, check=True)

def run(options):
    subprocess.run("qemu-system-x86_64 -bios bios.bin disk.fat -device qemu-xhci -device usb-kbd" + (" -s" if options.debug else ""))

if __name__ == "__main__":
    args = sys.argv
    print(args)
    task = args[1] if len(args) > 1 else "build"
    options = Options()
    for i in range(2, len(args)):
        if args[i] == "debug":
            options.debug = True
        elif args[i] == "release":
            options.debug = False
    options.os = platform.system().lower()

    if task == "build":
        build(options)
    elif task == "run":
        build(options)
        run(options)
    
