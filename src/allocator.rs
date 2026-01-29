use core::ptr;

// Align to 16 bytes as in heap.c
const HEAP_BLOCK_SIZE: usize = core::mem::size_of::<HeapBlock>();

#[repr(C)]
struct HeapBlock {
    size: usize,
    next: *mut HeapBlock,
    prev: *mut HeapBlock,
    free: bool,
}

static mut FREE_LIST: *mut HeapBlock = ptr::null_mut();

/// Initialize the heap
/// # Safety
/// Caller must ensure that [start, start + size) is valid memory.
pub unsafe fn init(start: usize, size: usize) {
    let block = start as *mut HeapBlock;
    unsafe {
        (*block).size = size - HEAP_BLOCK_SIZE;
        (*block).next = ptr::null_mut();
        (*block).prev = ptr::null_mut();
        (*block).free = true;
        FREE_LIST = block;
    }
    crate::println!("Heap initialized at {:#x} with size {}", start, size);
}

/// Allocate memory
/// # Safety
/// This function is unsafe because it manipulates raw pointers and global state.
pub unsafe fn alloc(size: usize) -> *mut u8 {
    let mut current = unsafe { FREE_LIST };

    // Simple First-Fit
    while !current.is_null() {
        unsafe {
            if (*current).free && (*current).size >= size {
                // Can we split the block?
                if (*current).size >= size + HEAP_BLOCK_SIZE + 16 {
                    let next_addr = (current as usize) + HEAP_BLOCK_SIZE + size;
                    let next = next_addr as *mut HeapBlock;

                    (*next).size = (*current).size - size - HEAP_BLOCK_SIZE;
                    (*next).next = (*current).next;
                    (*next).prev = current;
                    (*next).free = true;

                    if !(*next).next.is_null() {
                        (*(*next).next).prev = next;
                    }

                    (*current).size = size;
                    (*current).next = next;
                }

                (*current).free = false;
                return ((current as usize) + HEAP_BLOCK_SIZE) as *mut u8;
            }
            current = (*current).next;
        }
    }

    ptr::null_mut()
}

/// Free memory
/// # Safety
/// Caller must ensure ptr was allocated by alloc.
pub unsafe fn free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }

    let block = (ptr as usize - HEAP_BLOCK_SIZE) as *mut HeapBlock;
    unsafe {
        (*block).free = true;

        // Coalesce with next
        let next = (*block).next;
        if !next.is_null() && (*next).free {
            (*block).size += (*next).size + HEAP_BLOCK_SIZE;
            (*block).next = (*next).next;
            if !(*block).next.is_null() {
                (*(*block).next).prev = block;
            }
        }

        // Coalesce with prev
        let prev = (*block).prev;
        if !prev.is_null() && (*prev).free {
            (*prev).size += (*block).size + HEAP_BLOCK_SIZE;
            (*prev).next = (*block).next;
            if !(*block).next.is_null() {
                (*(*block).next).prev = prev;
            }
        }
    }
}

use core::alloc::{GlobalAlloc, Layout};

struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Pass size to our simple allocator
        unsafe { alloc(layout.size()) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { free(ptr) }
    }
}

#[global_allocator]
static ALLOCATOR: KernelAllocator = KernelAllocator;
