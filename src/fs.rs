#![allow(dead_code)]

use crate::nvme;

pub const BLOCK_SIZE: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    NotReady,
    InvalidArgument,
    DeviceError,
    NotFormatted,
    NoSpace,
    FileNotFound,
}

impl FsError {
    pub fn code(self) -> i32 {
        match self {
            FsError::NotReady => -1,
            FsError::InvalidArgument => -2,
            FsError::DeviceError => -3,
            FsError::NotFormatted => -4,
            FsError::NoSpace => -5,
            FsError::FileNotFound => -6,
        }
    }
}

pub type FsResult<T> = Result<T, FsError>;

pub fn is_ready() -> bool {
    unsafe { nvme::default_nsid().is_some() }
}

pub fn block_size() -> usize {
    BLOCK_SIZE
}

pub fn read_block(lba: u64, buffer: &mut [u8; BLOCK_SIZE]) -> FsResult<()> {
    read_blocks(lba, 1, buffer.as_mut_ptr())
}

pub fn write_block(lba: u64, buffer: &[u8; BLOCK_SIZE]) -> FsResult<()> {
    write_blocks(lba, 1, buffer.as_ptr())
}

pub fn read_blocks(lba: u64, count: u32, buffer: *mut u8) -> FsResult<()> {
    if count == 0 || buffer.is_null() {
        return Err(FsError::InvalidArgument);
    }

    let cs: u16;
    unsafe {
        core::arch::asm!("mov {0:x}, cs", out(reg) cs, options(nomem, nostack, preserves_flags));
    }
    let is_user = (cs & 0x03) == 3;

    if is_user {
        let ret = unsafe {
            crate::std::syscall(
                7, // sys_nvme_read
                lba as usize,
                buffer as usize,
                count as usize,
                0,
                0,
                0,
            )
        } as i32;
        if ret == 0 {
            Ok(())
        } else {
            Err(match ret {
                -1 => FsError::NotReady,
                -2 => FsError::InvalidArgument,
                -3 => FsError::DeviceError,
                -4 => FsError::NotFormatted,
                -5 => FsError::NoSpace,
                -6 => FsError::FileNotFound,
                _ => FsError::DeviceError,
            })
        }
    } else {
        let nsid = unsafe { nvme::default_nsid().ok_or(FsError::NotReady)? };
        let status = unsafe { nvme::nvme_read(nsid, lba, buffer, count) };
        if status == 0 {
            Ok(())
        } else {
            Err(FsError::DeviceError)
        }
    }
}

pub fn write_blocks(lba: u64, count: u32, buffer: *const u8) -> FsResult<()> {
    if count == 0 || buffer.is_null() {
        return Err(FsError::InvalidArgument);
    }

    let cs: u16;
    unsafe {
        core::arch::asm!("mov {0:x}, cs", out(reg) cs, options(nomem, nostack, preserves_flags));
    }
    let is_user = (cs & 0x03) == 3;

    if is_user {
        let ret = unsafe {
            crate::std::syscall(
                8, // sys_nvme_write
                lba as usize,
                buffer as usize,
                count as usize,
                0,
                0,
                0,
            )
        } as i32;
        if ret == 0 {
            Ok(())
        } else {
            Err(match ret {
                -1 => FsError::NotReady,
                -2 => FsError::InvalidArgument,
                -3 => FsError::DeviceError,
                -4 => FsError::NotFormatted,
                -5 => FsError::NoSpace,
                -6 => FsError::FileNotFound,
                _ => FsError::DeviceError,
            })
        }
    } else {
        let nsid = unsafe { nvme::default_nsid().ok_or(FsError::NotReady)? };
        let status = unsafe { nvme::nvme_write(nsid, lba, buffer as *mut u8, count) };
        if status == 0 {
            Ok(())
        } else {
            Err(FsError::DeviceError)
        }
    }
}

// ============================================================================
// SimpleFS Structures & Constants
// ============================================================================

pub const SUPERBLOCK_LBA: u64 = 0;
pub const DIR_START_LBA: u64 = 1;
pub const DIR_BLOCKS: u64 = 16;
pub const DATA_START_LBA: u64 = 1 + DIR_BLOCKS; // 17
pub const MAX_FILES: usize = (DIR_BLOCKS as usize) * 8; // 128 (8 entries of 64 bytes per block)
pub const SFS_MAGIC: u64 = 0x5349_4d50_4c45_4653; // "SIMPLEFS" in ASCII

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Superblock {
    pub magic: u64,           // Should match SFS_MAGIC
    pub next_free_block: u64, // The next block where file content can be written
    pub file_count: u32,      // Number of active files
    pub padding: [u8; 492],   // Pad to BLOCK_SIZE (512 bytes)
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct FileEntry {
    pub name: [u8; 47],       // Filename, null-terminated
    pub start_block: u64,     // The start block in NVMe
    pub size: u64,            // The file size in bytes
    pub in_use: u8,           // 1 if active, 0 if free
}

// ============================================================================
// SimpleFS Operations
// ============================================================================

pub fn read_superblock() -> FsResult<Superblock> {
    if !is_ready() {
        return Err(FsError::NotReady);
    }
    let mut buf = [0u8; BLOCK_SIZE];
    read_block(SUPERBLOCK_LBA, &mut buf)?;
    let sb = unsafe { core::ptr::read_unaligned(buf.as_ptr() as *const Superblock) };
    if sb.magic == SFS_MAGIC {
        Ok(sb)
    } else {
        Err(FsError::NotFormatted)
    }
}

pub fn write_superblock(sb: &Superblock) -> FsResult<()> {
    if !is_ready() {
        return Err(FsError::NotReady);
    }
    let mut buf = [0u8; BLOCK_SIZE];
    let sb_bytes = unsafe {
        core::slice::from_raw_parts(sb as *const Superblock as *const u8, BLOCK_SIZE)
    };
    buf.copy_from_slice(sb_bytes);
    write_block(SUPERBLOCK_LBA, &buf)
}

pub fn format() -> FsResult<()> {
    if !is_ready() {
        return Err(FsError::NotReady);
    }

    // Initialize superblock
    let sb = Superblock {
        magic: SFS_MAGIC,
        next_free_block: DATA_START_LBA,
        file_count: 0,
        padding: [0; 492],
    };
    write_superblock(&sb)?;

    // Zero out directory blocks
    let zero_buf = [0u8; BLOCK_SIZE];
    for b in 0..DIR_BLOCKS {
        write_block(DIR_START_LBA + b, &zero_buf)?;
    }

    Ok(())
}

pub fn write_directory_entry(index: usize, entry: &FileEntry) -> FsResult<()> {
    if index >= MAX_FILES {
        return Err(FsError::InvalidArgument);
    }
    let block_offset = index / 8;
    let entry_offset = (index % 8) * 64;
    let lba = DIR_START_LBA + (block_offset as u64);

    let mut buf = [0u8; BLOCK_SIZE];
    read_block(lba, &mut buf)?;

    let entry_bytes = unsafe {
        core::slice::from_raw_parts(entry as *const FileEntry as *const u8, 64)
    };
    buf[entry_offset..entry_offset + 64].copy_from_slice(entry_bytes);
    write_block(lba, &buf)
}

pub fn find_file(name: &str) -> FsResult<Option<(usize, FileEntry)>> {
    let name_bytes = name.as_bytes();
    if name_bytes.is_empty() || name_bytes.len() > 46 {
        return Err(FsError::InvalidArgument);
    }

    let mut buf = [0u8; BLOCK_SIZE];
    for b in 0..DIR_BLOCKS {
        read_block(DIR_START_LBA + b, &mut buf)?;
        for i in 0..8 {
            let offset = i * 64;
            let entry = unsafe {
                core::ptr::read_unaligned(buf[offset..].as_ptr() as *const FileEntry)
            };
            if entry.in_use == 1 {
                let mut len = 0;
                while len < 47 && entry.name[len] != 0 {
                    len += 1;
                }
                if &entry.name[..len] == name_bytes {
                    let index = (b as usize) * 8 + i;
                    return Ok(Some((index, entry)));
                }
            }
        }
    }
    Ok(None)
}

pub fn find_free_entry() -> FsResult<Option<usize>> {
    let mut buf = [0u8; BLOCK_SIZE];
    for b in 0..DIR_BLOCKS {
        read_block(DIR_START_LBA + b, &mut buf)?;
        for i in 0..8 {
            let offset = i * 64;
            let entry = unsafe {
                core::ptr::read_unaligned(buf[offset..].as_ptr() as *const FileEntry)
            };
            if entry.in_use == 0 {
                let index = (b as usize) * 8 + i;
                return Ok(Some(index));
            }
        }
    }
    Ok(None)
}

pub fn create_file(name: &str, data: &[u8]) -> FsResult<()> {
    let name_bytes = name.as_bytes();
    if name_bytes.is_empty() || name_bytes.len() > 46 {
        return Err(FsError::InvalidArgument);
    }

    let mut sb = read_superblock()?;

    // Check if the file already exists. If so, delete it first to overwrite.
    if let Some((idx, _old_entry)) = find_file(name)? {
        delete_file_at(idx)?;
        // Reload superblock after deletion
        sb = read_superblock()?;
    }

    // Find a free directory entry
    let free_idx = find_free_entry()?.ok_or(FsError::NoSpace)?;

    // Calculate blocks needed
    let blocks_needed = (data.len() + BLOCK_SIZE - 1) / BLOCK_SIZE;

    // Write data to blocks
    let start_block = sb.next_free_block;
    let mut block_buf = [0u8; BLOCK_SIZE];
    for i in 0..blocks_needed {
        let lba = start_block + (i as u64);
        let data_offset = i * BLOCK_SIZE;
        let data_len = (data.len() - data_offset).min(BLOCK_SIZE);

        block_buf[..data_len].copy_from_slice(&data[data_offset..data_offset + data_len]);
        if data_len < BLOCK_SIZE {
            block_buf[data_len..].fill(0);
        }

        write_block(lba, &block_buf)?;
    }

    // Construct the new directory entry
    let mut new_entry = FileEntry {
        name: [0; 47],
        start_block,
        size: data.len() as u64,
        in_use: 1,
    };
    new_entry.name[..name_bytes.len()].copy_from_slice(name_bytes);

    // Save the new directory entry
    write_directory_entry(free_idx, &new_entry)?;

    // Update the superblock
    sb.next_free_block += blocks_needed as u64;
    sb.file_count += 1;
    write_superblock(&sb)?;

    Ok(())
}

pub fn read_file(name: &str) -> FsResult<alloc::vec::Vec<u8>> {
    if let Some((_idx, entry)) = find_file(name)? {
        let size = entry.size as usize;
        let mut data = alloc::vec![0u8; size];
        if size == 0 {
            return Ok(data);
        }

        let blocks_to_read = (size + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let mut block_buf = [0u8; BLOCK_SIZE];
        for i in 0..blocks_to_read {
            let lba = entry.start_block + (i as u64);
            read_block(lba, &mut block_buf)?;

            let data_offset = i * BLOCK_SIZE;
            let data_len = (size - data_offset).min(BLOCK_SIZE);
            data[data_offset..data_offset + data_len].copy_from_slice(&block_buf[..data_len]);
        }

        Ok(data)
    } else {
        Err(FsError::FileNotFound)
    }
}

pub fn delete_file(name: &str) -> FsResult<()> {
    if let Some((idx, _entry)) = find_file(name)? {
        delete_file_at(idx)
    } else {
        Err(FsError::FileNotFound)
    }
}

fn delete_file_at(index: usize) -> FsResult<()> {
    let entry = FileEntry {
        name: [0; 47],
        start_block: 0,
        size: 0,
        in_use: 0,
    };
    write_directory_entry(index, &entry)?;

    let mut sb = read_superblock()?;
    if sb.file_count > 0 {
        sb.file_count -= 1;
        write_superblock(&sb)?;
    }
    Ok(())
}

pub struct PublicFileEntry {
    pub name: alloc::string::String,
    pub size: u64,
    pub start_block: u64,
}

pub fn list_files() -> FsResult<alloc::vec::Vec<PublicFileEntry>> {
    let _sb = read_superblock()?; // Ensure SFS is formatted
    let mut list = alloc::vec::Vec::new();
    let mut buf = [0u8; BLOCK_SIZE];
    for b in 0..DIR_BLOCKS {
        read_block(DIR_START_LBA + b, &mut buf)?;
        for i in 0..8 {
            let offset = i * 64;
            let entry = unsafe {
                core::ptr::read_unaligned(buf[offset..].as_ptr() as *const FileEntry)
            };
            if entry.in_use == 1 {
                let mut len = 0;
                while len < 47 && entry.name[len] != 0 {
                    len += 1;
                }
                let name = alloc::string::String::from_utf8_lossy(&entry.name[..len]).into_owned();
                list.push(PublicFileEntry {
                    name,
                    size: entry.size,
                    start_block: entry.start_block,
                });
            }
        }
    }
    Ok(list)
}
