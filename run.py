import os

def main():
    os.system("./build.ps1")
    os.system("qemu-system-x86_64.exe -bios bios.bin disk.fat -device qemu-xhci")