#ifndef EFI_H
#define EFI_H

#include <stdint.h>

#define EFIAPI __attribute__((ms_abi))

typedef void *EFI_HANDLE;
typedef uint64_t EFI_STATUS;
typedef uint64_t EFI_PHYSICAL_ADDRESS;
typedef uint64_t EFI_VIRTUAL_ADDRESS;
typedef uint64_t UINTN;

#define EFI_SUCCESS 0
#define EFI_ERR 0x8000000000000000
#define EFI_BUFFER_TOO_SMALL (EFI_ERR | 5)

typedef struct {
  uint32_t Data1;
  uint16_t Data2;
  uint16_t Data3;
  uint8_t Data4[8];
} EFI_GUID;

typedef struct {
  EFI_GUID VendorGuid;
  void *VendorTable;
} EFI_CONFIGURATION_TABLE;

// Protocols
typedef struct _EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL;
typedef struct _EFI_GRAPHICS_OUTPUT_PROTOCOL EFI_GRAPHICS_OUTPUT_PROTOCOL;
typedef struct _EFI_SYSTEM_TABLE EFI_SYSTEM_TABLE;

typedef EFI_STATUS(EFIAPI *EFI_TEXT_STRING)(
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL *This, uint16_t *String);

struct _EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL {
  void *Reset;
  EFI_TEXT_STRING OutputString;
};

// GOP
typedef struct {
  uint32_t RedMask;
  uint32_t GreenMask;
  uint32_t BlueMask;
  uint32_t ReservedMask;
} EFI_PIXEL_BITMASK;

typedef enum {
  PixelRedGreenBlueReserved8BitPerColor,
  PixelBlueGreenRedReserved8BitPerColor,
  PixelBitMask,
  PixelBltOnly,
  PixelFormatMax
} EFI_GRAPHICS_PIXEL_FORMAT;

typedef struct {
  uint32_t Version;
  uint32_t HorizontalResolution;
  uint32_t VerticalResolution;
  EFI_GRAPHICS_PIXEL_FORMAT PixelFormat;
  EFI_PIXEL_BITMASK PixelInformation;
  uint32_t PixelsPerScanLine;
} EFI_GRAPHICS_OUTPUT_MODE_INFORMATION;

typedef struct {
  uint32_t MaxMode;
  uint32_t Mode;
  EFI_GRAPHICS_OUTPUT_MODE_INFORMATION *Info;
  UINTN SizeOfInfo;
  EFI_PHYSICAL_ADDRESS FrameBufferBase;
  UINTN FrameBufferSize;
} EFI_GRAPHICS_OUTPUT_PROTOCOL_MODE;

struct _EFI_GRAPHICS_OUTPUT_PROTOCOL {
  void *QueryMode;
  void *SetMode;
  void *Blt;
  EFI_GRAPHICS_OUTPUT_PROTOCOL_MODE *Mode;
};

#define EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID                                      \
  {0x9042a9de, 0x23dc, 0x4a38, {0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a}}

#define EFI_LOADED_IMAGE_PROTOCOL_GUID                                         \
  {0x5B1B31A1, 0x9562, 0x11D2, {0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B}}

#define ACPI_20_TABLE_GUID                                                     \
  {0x8868e871, 0xe4f1, 0x11d3, {0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81}}

typedef struct {
  uint32_t Revision;
  EFI_HANDLE ParentHandle;
  EFI_SYSTEM_TABLE *SystemTable;
  EFI_HANDLE DeviceHandle;
  void *FilePath;
  void *Reserved;
  uint32_t LoadOptionsSize;
  void *LoadOptions;
  void *ImageBase;
  uint64_t ImageSize;
  uint32_t ImageCodeType;
  uint32_t ImageDataType;
  // ... other fields omitted
} EFI_LOADED_IMAGE_PROTOCOL;

// Memory types
typedef enum {
  EfiReservedMemoryType,
  EfiLoaderCode,
  EfiLoaderData,
  EfiBootServicesCode,
  EfiBootServicesData,
  EfiRuntimeServicesCode,
  EfiRuntimeServicesData,
  EfiConventionalMemory,
  EfiUnusableMemory,
  EfiACPIReclaimMemory,
  EfiACPIMemoryNVS,
  EfiMemoryMappedIO,
  EfiMemoryMappedIOPortSpace,
  EfiPalCode,
  EfiPersistentMemory,
  EfiMaxMemoryType
} EFI_MEMORY_TYPE;

// Memory Map
typedef struct {
  uint32_t Type;
  EFI_PHYSICAL_ADDRESS PhysicalStart;
  EFI_VIRTUAL_ADDRESS VirtualStart;
  uint64_t NumberOfPages;
  uint64_t Attribute;
} EFI_MEMORY_DESCRIPTOR;

// Boot Services
typedef struct {
  char Signature[8];
  uint32_t Revision;
  uint32_t HeaderSize;
  uint32_t CRC32;
  uint32_t Reserved;
} EFI_TABLE_HEADER;

typedef EFI_STATUS(EFIAPI *EFI_LOCATE_PROTOCOL)(EFI_GUID *Protocol,
                                                void *Registration,
                                                void **Interface);

typedef EFI_STATUS(EFIAPI *EFI_GET_MEMORY_MAP)(UINTN *MemoryMapSize,
                                               EFI_MEMORY_DESCRIPTOR *MemoryMap,
                                               UINTN *MapKey,
                                               UINTN *DescriptorSize,
                                               uint32_t *DescriptorVersion);

typedef EFI_STATUS(EFIAPI *EFI_EXIT_BOOT_SERVICES)(EFI_HANDLE ImageHandle,
                                                   UINTN MapKey);
typedef EFI_STATUS(EFIAPI *EFI_HANDLE_PROTOCOL)(EFI_HANDLE Handle,
                                                EFI_GUID *Protocol,
                                                void **Interface);
typedef struct {
  EFI_TABLE_HEADER Hdr;
  void *RaiseTPL;
  void *RestoreTPL;
  void *AllocatePages;
  void *FreePages;
  EFI_GET_MEMORY_MAP GetMemoryMap;
  void *AllocatePool;
  EFI_STATUS(EFIAPI *FreePool)(void *Buffer);
  void *CreateEvent;
  void *SetTimer;
  void *WaitForEvent;
  void *SignalEvent;
  void *CloseEvent;
  void *CheckEvent;
  void *InstallProtocolInterface;
  void *ReinstallProtocolInterface;
  void *UninstallProtocolInterface;
  EFI_HANDLE_PROTOCOL HandleProtocol;
  void *VoidReserved;
  void *RegisterProtocolNotify;
  EFI_STATUS(EFIAPI *LocateHandle)(uint32_t SearchType, EFI_GUID *Protocol,
                                   void *SearchKey, UINTN *BufferSize,
                                   EFI_HANDLE *Buffer);
  void *LocateDevicePath;
  void *InstallConfigurationTable;
  void *LoadImage;
  void *StartImage;
  void *Exit;
  void *UnloadImage;
  EFI_EXIT_BOOT_SERVICES ExitBootServices;
  void *GetNextMonotonicCount;
  void *Stall;
  void *SetWatchdogTimer;
  void *ConnectController;
  void *DisconnectController;
  void *OpenProtocol;
  void *CloseProtocol;
  void *OpenProtocolInformation;
  void *ProtocolsPerHandle;
  EFI_STATUS(EFIAPI *LocateHandleBuffer)(uint32_t SearchType,
                                         EFI_GUID *Protocol, void *SearchKey,
                                         UINTN *NoHandles, EFI_HANDLE **Buffer);
  EFI_LOCATE_PROTOCOL LocateProtocol;
  // ... rest omitted
} EFI_BOOT_SERVICES;

struct _EFI_SYSTEM_TABLE {
  EFI_TABLE_HEADER Hdr;
  uint16_t *FirmwareVendor;
  uint32_t FirmwareRevision;
  EFI_HANDLE ConsoleInHandle;
  void *ConIn;
  EFI_HANDLE ConsoleOutHandle;
  EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL *ConOut;
  EFI_HANDLE StandardErrorHandle;
  EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL *StdErr;
  void *RuntimeServices;
  EFI_BOOT_SERVICES *BootServices;
  UINTN NumberOfTableEntries;
  void *ConfigurationTable;
};
typedef struct _EFI_SYSTEM_TABLE EFI_SYSTEM_TABLE;

#endif
