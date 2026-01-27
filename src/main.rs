#![no_std]
#![no_main]

mod uefi;
use uefi::*;
use core::ffi::c_void;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BootInfo {
    pub framebuffer_base: u64,
    pub framebuffer_size: usize,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixels_per_scanline: u32,
    pub pixel_format: u32, // Simplified Enum mapping
    pub memory_map: *mut u8,
    pub memory_map_size: usize,
    pub descriptor_size: usize,
    pub descriptor_version: u32,
}

mod gdt;
mod memory;
mod interrupts;

mod writer;
use core::fmt::Write;

#[unsafe(no_mangle)]
pub extern "sysv64" fn kernel_main(boot_info: &BootInfo) -> ! {
    let mut writer = writer::Writer::new(*boot_info);

    let _ = writeln!(writer, "Hello World from Kernel!");
    let _ = writeln!(writer, "Resolution: {}x{}", boot_info.horizontal_resolution, boot_info.vertical_resolution);
    let _ = writeln!(writer, "Framebuffer: {:#x}", boot_info.framebuffer_base);

    // Initialize Frame Allocator
    let mut allocator = unsafe { memory::FrameAllocator::new(boot_info) };

    // Initialize Global Writer (for interrupts)
    unsafe {
        writer::init_global_writer(*boot_info);
    }

    // Initialize GDT
    unsafe {
        gdt::init();
        interrupts::init_idt();
        let _ = writeln!(writer, "GDT & IDT Initialized!");
    }

    unsafe {
        memory::init_paging(boot_info, &mut allocator);
        let _ = writeln!(writer, "Paging Initialized!");
    }

    // Test Allocation
    for i in 0..5 {
        if let Some(frame) = allocator.allocate_frame() {
            let _ = writeln!(writer, "Allocated Frame {}: {:#x}", i, frame);
        } else {
            let _ = writeln!(writer, "Failed to allocate frame {}", i);
        }
    }

    // Switch to User Mode
    unsafe {
        let _ = writeln!(writer, "Switching to User Mode...");
        enter_usermode();
    }
    //it is unreachable code if it successfully switches to user mode
    let _ = writeln!(writer, "failed to switch to user mode");
    loop {}
}

#[unsafe(no_mangle)]
pub unsafe extern "sysv64" fn user_main() {
    loop {
        // Spin in user mode
    }
}

pub unsafe fn enter_usermode() -> ! {
    let user_cs: u64 = gdt::USER_CODE_SEL as u64; 
    let user_ds: u64 = gdt::USER_DATA_SEL as u64; 
    let user_rsp = 0x100000u64; 
    
    use core::arch::asm;
    
    // RFLAGS: Interrupts enabled (0x200) | Reserved (0x2) = 0x202
    let rflags: u64 = 0x202;
    let rip = user_main as u64;

    unsafe {
        asm!(
            "push {ds}",       // SS
            "push {rsp}",      // RSP
            "push {rflags}",   // RFLAGS
            "push {cs}",       // CS
            "push {rip}",      // RIP
            "iretq",
            ds = in(reg) user_ds,
            rsp = in(reg) user_rsp,
            rflags = in(reg) rflags,
            cs = in(reg) user_cs,
            rip = in(reg) rip,
            options(noreturn)
        );
    }
}

#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(_image_handle: EFI_HANDLE, system_table: *mut EFI_SYSTEM_TABLE) -> EFI_STATUS {
    // 1. Initialize formatted output (minimal)
    let msg = "Getting ready to jump to kernel...\r\n\0";
    let mut buffer: [u16; 64] = [0; 64];
    for (i, b) in msg.bytes().enumerate() {
        if i >= 63 { break; }
        buffer[i] = b as u16;
    }
    
    unsafe {
        let con_out = (*system_table).ConOut;
        ((*con_out).OutputString)(con_out, buffer.as_ptr());
    }

    let boot_services = unsafe { (*system_table).BootServices };

    // 2. Locate GOP
    let mut gop: *mut EFI_GRAPHICS_OUTPUT_PROTOCOL = core::ptr::null_mut();
    let gop_guid = EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID;
    
    let status = unsafe {
        ((*boot_services).LocateProtocol)(
            &gop_guid as *const EFI_GUID,
            core::ptr::null_mut(),
            &mut gop as *mut *mut EFI_GRAPHICS_OUTPUT_PROTOCOL as *mut *mut c_void
        )
    };

    if status != 0 {
        // Failed to locate GOP
        return status;
    }

    // 3. Prepare BootInfo
    let mode = unsafe { *(*gop).Mode };
    let info = unsafe { *mode.Info };
    
    // Framebuffer might need mapping? In UEFI it is identity mapped or IO mapped.
    // We assume we can access it directly for now (x86_64 UEFI usually maps it).
    
    let framebuffer_base = mode.FrameBufferBase;
    let framebuffer_size = mode.FrameBufferSize;
    let horizontal_resolution = info.HorizontalResolution;
    let vertical_resolution = info.VerticalResolution;
    let pixels_per_scanline = info.PixelsPerScanLine;
    let pixel_format = info.PixelFormat as u32;

    // 4. Get Memory Map
    // We need a larger buffer for real hardware.
    // Using static buffer to avoid stack overflow or allocation issues.
    // But since no global allocator, we put it on stack or use raw bytes.
    // 16KB should be enough.
    let mut memory_map_buffer = [0u8; 16384]; 
    let mut memory_map_size = memory_map_buffer.len();
    let mut map_key: usize = 0;
    let mut descriptor_size: usize = 0;
    let mut descriptor_version: u32 = 0;

    let memory_map_ptr = memory_map_buffer.as_mut_ptr() as *mut EFI_MEMORY_DESCRIPTOR;

    let status = unsafe {
        ((*boot_services).GetMemoryMap)(
            &mut memory_map_size,
            memory_map_ptr,
            &mut map_key,
            &mut descriptor_size,
            &mut descriptor_version
        )
    };

    if status != 0 {
        return status;
    }

    // 5. Exit Boot Services
    let mut status = unsafe { ((*boot_services).ExitBootServices)(_image_handle, map_key) };

    if status != 0 {
        // The memory map changed between GetMemoryMap and ExitBootServices.
        // We must get the memory map again and retry once.
        memory_map_size = memory_map_buffer.len();
        status = unsafe {
            ((*boot_services).GetMemoryMap)(
                &mut memory_map_size,
                memory_map_ptr,
                &mut map_key,
                &mut descriptor_size,
                &mut descriptor_version
            )
        };

        if status != 0 {
            return status;
        }

        status = unsafe { ((*boot_services).ExitBootServices)(_image_handle, map_key) };

        if status != 0 {
            return status;
        }
    }

    // 6. Jump to Kernel
    let boot_info = BootInfo {
        framebuffer_base,
        framebuffer_size,
        horizontal_resolution,
        vertical_resolution,
        pixels_per_scanline,
        pixel_format,
        memory_map: memory_map_ptr as *mut u8,
        memory_map_size,
        descriptor_size,
        descriptor_version,
    };

    kernel_main(&boot_info);
}
