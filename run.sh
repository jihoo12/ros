cargo build --target x86_64-unknown-uefi
mkdir -p esp/EFI/BOOT
cp target/x86_64-unknown-uefi/debug/os.efi esp/EFI/BOOT/BOOTX64.EFI
if [ ! -f nvme.img ]; then
    qemu-img create -f raw nvme.img 1G
fi

qemu-system-x86_64 \
    -bios /usr/share/ovmf/OVMF.fd \
    -drive format=raw,file=fat:rw:esp \
    -drive file=nvme.img,if=none,id=nvm,format=raw \
    -device nvme,serial=deadbeef,drive=nvm \
    -device qemu-xhci,id=xhci,msi=off,msix=off \
    -net none \
    -serial stdio \
    -d int,cpu_reset \
    -no-reboot \
    -D qemu.log
