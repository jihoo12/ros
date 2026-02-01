# Rust UEFI OS

A custom Operating System written in Rust, targeting the x86_64 UEFI architecture. This project demonstrates key OS concepts including UEFI booting, graphical user interface, user-mode execution, system calls, and device driver support (NVMe & xHCI).

![Rust](https://img.shields.io/badge/language-Rust-orange)
![Platform](https://img.shields.io/badge/platform-x86__64--UEFI-blue)

## ✨ Features

- **UEFI Booting**: Fully compliant with the Unified Extensible Firmware Interface standard.
- **Graphical Framebuffer**: High-resolution graphics.
- **Interactive Shell**: A built-in userspace shell (Ring 3) for command execution.
- **USB 3.0 Support**: Custom **xHCI Driver** supporting keyboard input.
- **NVMe Support**: Native driver for generic NVMe SSDs.
- **User Mode**: Secure transition from Kernel to User mode with Ring 3 privilege isolation.
- **System Calls**: Robust syscall interface for user-kernel communication (print, keyboard, shutdown, etc.).

## 🛠️ Prerequisites

To build and run this OS, you need the following tools installed:

- **Rust Nightly**: Required for experimental OS features (inline assembly, naked functions, etc.).
- **QEMU**: For system emulation (`qemu-system-x86_64`).
- **OVMF**: UEFI firmware for QEMU.
- **Python 3**: For generating the test framebuffer image.
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
4. Launch QEMU with the OS, USB keyboard, and NVMe drive attached.

### 3. Usage

Once the system boots, you will be dropped into an interactive shell.
**Available Commands:**
- `help`: Show available commands.
- `echo <args>`: Print arguments to the screen.
- `history`: Show command history.
- `clear`: Clear the screen.
- `shutdown`: Cleanly shut down the system (powers off QEMU).

## 📁 Project Structure

- `src/main.rs`: Kernel entry point and initialization.
- `src/uefi.rs`: UEFI definitions and bindings.
- `src/gdt.rs` / `src/interrupts.rs`: CPU descriptor tables and interrupt handling.
- `src/memory.rs` / `src/allocator.rs`: Memory management and paging.
- `src/nvme.rs`: NVMe storage driver.
- `src/xhci.rs`: USB 3.0 xHCI host controller driver.
- `src/shell.rs`: Userspace shell implementation.
- `src/syscall.rs`: System call handlers.
- `src/writer.rs`: Graphics rendering with double buffering.


