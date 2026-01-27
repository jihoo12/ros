#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use core::ffi::c_void;

// Basic Types
pub type EFI_STATUS = usize;
pub type EFI_HANDLE = *mut c_void;
pub type CHAR16 = u16;

#[repr(C)]
pub struct EFI_GUID {
    pub Data1: u32,
    pub Data2: u16,
    pub Data3: u16,
    pub Data4: [u8; 8],
}

pub const EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID: EFI_GUID = EFI_GUID {
    Data1: 0x9042a9de,
    Data2: 0x23dc,
    Data3: 0x4a38,
    Data4: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct EFI_PIXEL_BITMASK {
    pub RedMask: u32,
    pub GreenMask: u32,
    pub BlueMask: u32,
    pub ReservedMask: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub enum EFI_GRAPHICS_PIXEL_FORMAT {
    PixelRedGreenBlueReserved8BitPerColor,
    PixelBlueGreenRedReserved8BitPerColor,
    PixelBitMask,
    PixelBltOnly,
    PixelFormatMax,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct EFI_GRAPHICS_OUTPUT_MODE_INFORMATION {
    pub Version: u32,
    pub HorizontalResolution: u32,
    pub VerticalResolution: u32,
    pub PixelFormat: EFI_GRAPHICS_PIXEL_FORMAT,
    pub PixelInformation: EFI_PIXEL_BITMASK,
    pub PixelsPerScanLine: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct EFI_GRAPHICS_OUTPUT_PROTOCOL_MODE {
    pub MaxMode: u32,
    pub Mode: u32,
    pub Info: *mut EFI_GRAPHICS_OUTPUT_MODE_INFORMATION,
    pub SizeOfInfo: usize,
    pub FrameBufferBase: u64, // EFI_PHYSICAL_ADDRESS
    pub FrameBufferSize: usize,
}

#[repr(C)]
pub struct EFI_GRAPHICS_OUTPUT_PROTOCOL {
    pub QueryMode: *const c_void,
    pub SetMode: *const c_void,
    pub Blt: *const c_void,
    pub Mode: *mut EFI_GRAPHICS_OUTPUT_PROTOCOL_MODE,
}



// Protocols
#[repr(C)]
pub struct EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL {
    pub Reset: unsafe extern "efiapi" fn(This: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL, ExtendedVerification: bool) -> EFI_STATUS,
    pub OutputString: unsafe extern "efiapi" fn(This: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL, String: *const CHAR16) -> EFI_STATUS,
    // Other fields omitted for minimalism as we only need OutputString
}

// Boot Services
#[repr(C)]
pub struct EFI_TABLE_HEADER {
    pub Signature: u64,
    pub Revision: u32,
    pub HeaderSize: u32,
    pub CRC32: u32,
    pub Reserved: u32,
}

// Memory Map
#[repr(C)]
pub struct EFI_MEMORY_DESCRIPTOR {
  pub Type: u32,
  pub PhysicalStart: u64, // EFI_PHYSICAL_ADDRESS
  pub VirtualStart: u64, // EFI_VIRTUAL_ADDRESS
  pub NumberOfPages: u64,
  pub Attribute: u64,
}

pub const EFI_RESERVED_MEMORY_TYPE: u32 = 0;
pub const EFI_LOADER_CODE: u32 = 1;
pub const EFI_LOADER_DATA: u32 = 2;
pub const EFI_BOOT_SERVICES_CODE: u32 = 3;
pub const EFI_BOOT_SERVICES_DATA: u32 = 4;
pub const EFI_RUNTIME_SERVICES_CODE: u32 = 5;
pub const EFI_RUNTIME_SERVICES_DATA: u32 = 6;
pub const EFI_CONVENTIONAL_MEMORY: u32 = 7;
pub const EFI_UNUSABLE_MEMORY: u32 = 8;
pub const EFI_ACPI_RECLAIM_MEMORY: u32 = 9;
pub const EFI_ACPI_MEMORY_NVS: u32 = 10;
pub const EFI_MEMORY_MAPPED_IO: u32 = 11;
pub const EFI_MEMORY_MAPPED_IO_PORT_SPACE: u32 = 12;
pub const EFI_PAL_CODE: u32 = 13;
pub const EFI_PERSISTENT_MEMORY: u32 = 14;

#[repr(C)]
pub struct EFI_BOOT_SERVICES {
    pub Hdr: EFI_TABLE_HEADER,
    pub RaiseTPL: *const c_void,
    pub RestoreTPL: *const c_void,
    pub AllocatePages: *const c_void,
    pub FreePages: *const c_void,
    pub GetMemoryMap: unsafe extern "efiapi" fn(
        MemoryMapSize: *mut usize,
        MemoryMap: *mut EFI_MEMORY_DESCRIPTOR,
        MapKey: *mut usize,
        DescriptorSize: *mut usize,
        DescriptorVersion: *mut u32
    ) -> EFI_STATUS,
    pub AllocatePool: *const c_void,
    pub FreePool: *const c_void,
    pub CreateEvent: *const c_void,
    pub SetTimer: *const c_void,
    pub WaitForEvent: *const c_void,
    pub SignalEvent: *const c_void,
    pub CloseEvent: *const c_void,
    pub CheckEvent: *const c_void,
    pub InstallProtocolInterface: *const c_void,
    pub ReinstallProtocolInterface: *const c_void,
    pub UninstallProtocolInterface: *const c_void,
    pub HandleProtocol: *const c_void,
    pub Reserved: *const c_void,
    pub RegisterProtocolNotify: *const c_void,
    pub LocateHandle: *const c_void,
    pub LocateDevicePath: *const c_void,
    pub InstallConfigurationTable: *const c_void,
    pub LoadImage: *const c_void,
    pub StartImage: *const c_void,
    pub Exit: *const c_void,
    pub UnloadImage: *const c_void,
    pub ExitBootServices: unsafe extern "efiapi" fn(ImageHandle: EFI_HANDLE, MapKey: usize) -> EFI_STATUS,
    pub GetNextMonotonicCount: *const c_void,
    pub Stall: *const c_void,
    pub SetWatchdogTimer: *const c_void,
    pub ConnectController: *const c_void,
    pub DisconnectController: *const c_void,
    pub OpenProtocol: *const c_void,
    pub CloseProtocol: *const c_void,
    pub OpenProtocolInformation: *const c_void,
    pub ProtocolsPerHandle: *const c_void,
    pub LocateHandleBuffer: *const c_void,
    pub LocateProtocol: unsafe extern "efiapi" fn(
        Protocol: *const EFI_GUID,
        Registration: *mut c_void,
        Interface: *mut *mut c_void,
    ) -> EFI_STATUS,
    pub InstallMultipleProtocolInterfaces: *const c_void,
    pub UninstallMultipleProtocolInterfaces: *const c_void,
    // Remaining fields omitted
}

#[repr(C)]
pub struct EFI_SYSTEM_TABLE {
    pub Hdr: EFI_TABLE_HEADER,
    pub FirmwareVendor: *const CHAR16,
    pub FirmwareRevision: u32,
    pub ConsoleInHandle: EFI_HANDLE,
    pub ConIn: *mut c_void,
    pub ConsoleOutHandle: EFI_HANDLE,
    pub ConOut: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
    pub StandardErrorHandle: EFI_HANDLE,
    pub StdErr: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
    pub RuntimeServices: *mut c_void,
    pub BootServices: *mut EFI_BOOT_SERVICES,
    pub NumberOfTableEntries: usize,
    pub ConfigurationTable: *mut c_void,
}
