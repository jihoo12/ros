
use crate::uefi::EFI_MEMORY_DESCRIPTOR;
use crate::BootInfo;

pub const PAGE_SIZE: u64 = 4096;

/// A simple physical frame allocator that uses the UEFI memory map.
pub struct FrameAllocator {
    memory_map: *const u8,
    memory_map_size: usize,
    pub descriptor_size: usize,
    pub _descriptor_version: u32,
    
    // State to track next allocation
    current_descriptor_index: usize,
    current_page_offset: u64, // Offset in pages within the current descriptor
}

impl FrameAllocator {
    /// Create a new FrameAllocator from the BootInfo.
    /// 
    /// # Safety
    /// The caller must ensure that the memory map passed in BootInfo is valid.
    pub unsafe fn new(boot_info: &BootInfo) -> Self {
        Self {
            memory_map: boot_info.memory_map,
            memory_map_size: boot_info.memory_map_size,
            descriptor_size: boot_info.descriptor_size,
            _descriptor_version: boot_info.descriptor_version,
            current_descriptor_index: 0,
            current_page_offset: 0,
        }
    }

    /// Allocate a single physical frame (4KB).
    /// Returns the physical address of the frame if successful, or None if no memory is available.
    pub fn allocate_frame(&mut self) -> Option<u64> {
        let num_descriptors = self.memory_map_size / self.descriptor_size;

        while self.current_descriptor_index < num_descriptors {
            let offset = self.current_descriptor_index * self.descriptor_size;
            let descriptor_ptr = unsafe { self.memory_map.add(offset) } as *const EFI_MEMORY_DESCRIPTOR;
            let descriptor = unsafe { &*descriptor_ptr };

            // EFI_CONVENTIONAL_MEMORY is typically type 7.
            // We should use the constant if available, but for now we look for 7.
            // Reference: UEFI Spec 2.9, Section 7.2
            // EfiConventionalMemory = 7
            if descriptor.Type == 7 {
                if self.current_page_offset < descriptor.NumberOfPages {
                    // Found a free page
                    let frame_address = descriptor.PhysicalStart + (self.current_page_offset * PAGE_SIZE);
                    
                    // Advance pointer for next time
                    self.current_page_offset += 1;

                    // Ensure the frame is not null (unlikely for usable memory, but good sanity check)
                    if frame_address > 0 {
                        return Some(frame_address);
                    }
                }
            }

            // Move to next descriptor
            self.current_descriptor_index += 1;
            self.current_page_offset = 0;
        }

        None
    }
}
