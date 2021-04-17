
import subprocess
import sys
import platform
import json

class Options:
    def __init__(self):
        self.os = None
        self.debug = False

def build(options):
    cargo_command_bootloader = ["cargo", "build", "--release"]
    cargo_command_kernel = ["cargo", "build"]
    if options.debug == False:
        cargo_command_kernel.append("--release")
    if options.debug:
        cargo_command_bootloader.append("--features=wait_for_gdb")
    subprocess.run(cargo_command_bootloader, cwd = "./bootloader", check=True)
    subprocess.run(cargo_command_kernel, cwd = "./kernel", check=True)
    pathpart = "debug" if options.debug else "release"
    commands = [
        ["dd",  "if=/dev/zero", "of=disk.fat", "bs=1M", "count=100"],
        ["sudo", "mkfs.vfat",  "disk.fat"],
        ["mmd", "-i", "disk.fat", "::EFI"],
        ["mmd", "-i", "disk.fat", "::EFI/BOOT"],
        ["mcopy",  "-i",  "disk.fat", "target/x86_64-unknown-uefi/release/bootloader.efi", "::EFI/BOOT/BOOTX64.EFI"],
        ["mcopy", "-i", "disk.fat", "target/target/"+pathpart+"/kernel", "::kernel.elf"]
    ]
    if options.os == "windows":
        for command in commands:
            subprocess.run("wsl -- " + " ".join(command), check=True)
    else:
        for command in commands:
            subprocess.run(command, check=True)

def run(options):
    command = ["qemu-system-x86_64", "-bios", "bios.bin", "disk.fat", "-device", "qemu-xhci"]
    if options.debug:
        command.append("-s")
    if options.os == "windows":
        command = " ".join(command)
    subprocess.run(command)

def install(options):
    subprocess.run(["cp",  "target/x86_64-unknown-uefi/release/bootloader.efi", "/run/media/elekrisk/6D95-4DD4/EFI/BOOT/BOOTX64.EFI"])
    pathpart = "debug" if options.debug else "release"
    subprocess.run(["cp",  "target/target/"+pathpart+"/kernel", "/run/media/elekrisk/6D95-4DD4/kernel.elf"])

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

    try:
        if task == "build":
            build(options)
        elif task == "run":
            build(options)
            run(options)
        elif task == "install":
            build(options)
            install(options)
    except subprocess.CalledProcessError:
        pass
    
