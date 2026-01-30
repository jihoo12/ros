# Rust UEFI OS

A custom Operating System written in Rust, targeting the x86_64 UEFI architecture. This project demonstrates key OS concepts including UEFI booting, graphical user interface, user-mode execution, system calls, and NVMe driver support.

![Rust](https://img.shields.io/badge/language-Rust-orange)
![Platform](https://img.shields.io/badge/platform-x86__64--UEFI-blue)

## ✨ Features

- **UEFI Booting**: Uses the UEFI Standard for booting.
- **Grafical Framebuffer**: Supports high-resolution graphics
- **NVMe Driver**: Custom NVMe driver implementation for high-speed storage access.
- **User Mode (Ring 3)**: Secure transition from Kernel to User mode.
- **System Calls**: Implemented syscall interface for user-kernel communication.


## 🛠️ Prerequisites

To build and run this OS, you need the following tools installed:

- **Rust Nightly**: Required for experimental OS features.
- **QEMU**: For system emulation.
- **OVMF**: UEFI firmware for QEMU.
- **Python 3**: For generating the test image.
- **PIL/Pillow**: Python library for image processing.

## 🚀 Getting Started

### 1. Prepare Test Image

The OS includes a feature to display an image on the screen. You must first generate the `image.bin` file.

1. Place your desired image (e.g., `image.jpg`) in the project root.
2. Run the image processing script:

```bash
python3 image.py
```

This will create `image.bin` which is raw pixel data formatted for the UEFI framebuffer (BGRA).

### 2. Build and Run

Use the provided helper script to compile the kernel, create the disk image, and launch QEMU.

```bash
./run.sh
```

This script will:
1. Build the kernel for `x86_64-unknown-uefi`.
2. Create the necessary EFI directory structure.
3. Create a raw NVMe disk image (`nvme.img`) if it doesn't exist.
4. Launch QEMU with the OS and NVMe drive attached.

## 📁 Project Structure

- `src/main.rs`: Kernel entry point and initialization.
- `src/uefi.rs`: UEFI definitions and bindings.
- `src/gdt.rs`: Global Descriptor Table setup.
- `src/interrupts.rs`: Interrupt Descriptor Table and handlers.
- `src/memory.rs`: Memory management and paging.
- `src/nvme.rs`: NVMe driver implementation.
- `src/writer.rs`: Graphics and text rendering.
- `src/syscall.rs`: System call handlers.

