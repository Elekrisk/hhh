cd bootloader
cargo build --release
cd ../kernel
wsl -e ../build2.sh
cd ..
wsl -- ./build1.sh