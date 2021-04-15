#!/bin/env python3

import subprocess
import sys
import os

def main():
    if os.system("cd bootloader && cargo build --release") != 0:
        return
    if os.system("cd kernel && cargo build --release") != 0:
        return
    if os.system("wsl -- ./build1.sh") != 0:
        return

if __name__ == "__main__":
    main()