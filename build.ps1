cd bootloader
cargo build --release
# if ($LastErrorCode -ne 0) {
#     cd ..
#     exit
# }
cd ../kernel
cargo build --release
# if ($LastErrorCode -ne 0) {
#     cd ..
#     exit
# }
cd ..
wsl -- ./build1.sh