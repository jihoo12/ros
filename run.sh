cargo build --target x86_64-unknown-uefi
mkdir -p esp/EFI/BOOT
cp target/x86_64-unknown-uefi/debug/os.efi esp/EFI/BOOT/BOOTX64.EFI
qemu-system-x86_64 \
    -bios /usr/share/ovmf/OVMF.fd \
    -drive format=raw,file=fat:rw:esp \
    -net none