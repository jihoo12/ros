use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

// Align to 16 bytes for HeapBlock headers
const HEAP_BLOCK_SIZE: usize = mem::size_of::<HeapBlock>();

#[repr(C)]
struct HeapBlock {
    size: usize,
    next: *mut HeapBlock,
    prev: *mut HeapBlock,
    free: bool,
}

struct LinkedListAllocator {
    head: *mut HeapBlock,
}

unsafe impl Send for LinkedListAllocator {}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        Self {
            head: ptr::null_mut(),
        }
    }

    pub unsafe fn init(&mut self, start: usize, size: usize) {
        let block = start as *mut HeapBlock;
        unsafe {
            (*block).size = size - HEAP_BLOCK_SIZE;
            (*block).next = ptr::null_mut();
            (*block).prev = ptr::null_mut();
            (*block).free = true;
            self.head = block;
        }
        crate::println!("Heap initialized at {:#x} with size {}", start, size);
    }

    pub unsafe fn alloc_raw(&mut self, mut size: usize) -> *mut u8 {
        // Ensure size is aligned to 8 bytes to keep block headers aligned
        let align_mask = mem::align_of::<usize>() - 1;
        size = (size + align_mask) & !align_mask;

        let mut current = self.head;

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
                    // Return the payload address (immediately after block header)
                    return ((current as usize) + HEAP_BLOCK_SIZE) as *mut u8;
                }
                current = (*current).next;
            }
        }
        ptr::null_mut()
    }

    pub unsafe fn free_raw(&mut self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }

        let block = (ptr as usize - HEAP_BLOCK_SIZE) as *mut HeapBlock;
        unsafe {
            if (*block).free {
                // Double free?
                return;
            }
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
}

// Spinlock Implementation
pub struct Spinlock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Spinlock<T> {}
unsafe impl<T: Send> Send for Spinlock<T> {}

impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> SpinlockGuard<T> {
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinlockGuard { lock: self }
    }
}

pub struct SpinlockGuard<'a, T> {
    lock: &'a Spinlock<T>,
}

impl<'a, T> Deref for SpinlockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> DerefMut for SpinlockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for SpinlockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.lock.store(false, Ordering::Release);
    }
}

// Global Kernel Allocator Wrapper
pub struct KernelAllocator;

// The static that holds the actual state (private)
static INNER_ALLOCATOR: Spinlock<LinkedListAllocator> = Spinlock::new(LinkedListAllocator::new());

// The static that implements GlobalAlloc (public)
#[global_allocator]
static ALLOCATOR: KernelAllocator = KernelAllocator;

// Helper to check Current Privilege Level
#[inline(always)]
fn get_cpl() -> u16 {
    let cs: u16;
    unsafe {
        core::arch::asm!("mov {0:x}, cs", out(reg) cs);
    }
    cs & 0x03
}

// Minimal syscall wrapper for allocation
#[inline(always)]
unsafe fn user_alloc(layout: Layout) -> *mut u8 {
    let ret: usize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 2, // sys_alloc
            in("rdi") layout.size(),
            in("rsi") layout.align(),
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, preserves_flags)
        );
    }
    ret as *mut u8
}

#[inline(always)]
unsafe fn user_free(ptr: *mut u8) {
    let _ret: usize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 3, // sys_free
            in("rdi") ptr,
            lateout("rax") _ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, preserves_flags)
        );
    }
}

#[inline(always)]
unsafe fn user_realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    let ret: usize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 13, // sys_realloc
            in("rdi") ptr,
            in("rsi") new_size,
            in("rdx") layout.align(),
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, preserves_flags)
        );
    }
    ret as *mut u8
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Check if we are in User Mode (Ring 3)
        if get_cpl() == 3 {
            return user_alloc(layout);
        }

        // We always use the "offset header" strategy to support alignment
        // and robust free.
        // Format: [HeapBlock] ... [OffsetHeader] [AlignedPayload]
        // OffsetHeader stores the address of [HeapBlock].

        let align = layout.align();
        let size = layout.size();

        // We reserve extra space for:
        // 1. Possible alignment padding (max align - 1)
        // 2. The OffsetHeader (sizeof(usize))
        // We ensure OffsetHeader is stored immediately before AlignedPayload.
        let offset_header_size = mem::size_of::<usize>();
        let extra_alloc = align + offset_header_size;

        let mut allocator = INNER_ALLOCATOR.lock();
        let ptr = unsafe { allocator.alloc_raw(size + extra_alloc) };

        if ptr.is_null() {
            return ptr::null_mut();
        }

        let addr = ptr as usize;
        // The alloc_raw returns Ptr which is Block + HEAP_BLOCK_SIZE.
        // Block = ptr - HEAP_BLOCK_SIZE.
        // We need to store Block address in the OffsetHeader.
        let block_addr = addr - HEAP_BLOCK_SIZE;

        // Calculate aligned address
        // We need:
        // aligned_ptr % align == 0
        // aligned_ptr >= addr + offset_header_size
        let start_candidate = addr + offset_header_size;
        let remainder = start_candidate % align;
        let padding = if remainder == 0 { 0 } else { align - remainder };

        let aligned_addr = start_candidate + padding;
        let aligned_ptr = aligned_addr as *mut u8;

        // Write the OffsetHeader immediately before aligned_ptr
        let header_ptr = (aligned_addr - offset_header_size) as *mut usize;
        unsafe {
            ptr::write(header_ptr, block_addr);
        }

        aligned_ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }

        if get_cpl() == 3 {
            user_free(ptr);
            return;
        }

        // Recover the block address
        let offset_header_size = mem::size_of::<usize>();
        let header_ptr = (ptr as usize - offset_header_size) as *mut usize;
        let block_addr = unsafe { ptr::read(header_ptr) };

        // Calculate the raw pointer returned by alloc_raw
        // alloc_raw returned block_addr + HEAP_BLOCK_SIZE
        let raw_ptr = (block_addr + HEAP_BLOCK_SIZE) as *mut u8;

        unsafe { INNER_ALLOCATOR.lock().free_raw(raw_ptr) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if get_cpl() == 3 {
            return user_realloc(ptr, layout, new_size);
        }

        if ptr.is_null() {
            return unsafe {
                self.alloc(Layout::from_size_align_unchecked(new_size, layout.align()))
            };
        }

        // We can try to optimize by growing in place if possible,
        // OR just fallback to alloc+copy+free for simplicity and correctness with alignment.
        // Given complexity of aligned headers, in-place growth is tricky if alignment padding changes.
        // For safely, let's use the default alloc-copy-free strategy but ensuring strict alignment.
        // The default GlobalAlloc::realloc does exactly this, but let's be explicit or implement optimization later.

        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        let new_ptr = unsafe { self.alloc(new_layout) };
        if !new_ptr.is_null() {
            // We need to know the OLD size to copy.
            // Recover block info.
            let offset_header_size = mem::size_of::<usize>();
            let header_ptr = (ptr as usize - offset_header_size) as *mut usize;
            let block_addr = unsafe { ptr::read(header_ptr) };
            let block = block_addr as *mut HeapBlock;
            let raw_payload_size = unsafe { (*block).size };
            // Original capacity calculation (approximate because padding is inside block payload area effectively)
            // Block Payload = [ ... padding ... | offset_header | aligned_payload ... ]
            // We know aligned_payload starts at ptr.
            // Block starts at block_addr.
            // Block Payload ends at block_addr + HEAP_BLOCK_SIZE + raw_payload_size.
            let raw_payload_end = block_addr + HEAP_BLOCK_SIZE + raw_payload_size;
            let old_capacity = raw_payload_end - (ptr as usize);

            let copy_size = if new_size < old_capacity {
                new_size
            } else {
                old_capacity
            };
            unsafe {
                ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
                self.dealloc(ptr, layout);
            }
        }
        new_ptr
    }
}

// Legacy / Helper Public API
// These functions are used by the rest of the kernel and syscalls (sys_alloc).

/// Initialize the heap
pub unsafe fn init(start: usize, size: usize) {
    unsafe { INNER_ALLOCATOR.lock().init(start, size) };
}

/// Allocate memory (Default alignment = 8)
pub unsafe fn alloc(size: usize) -> *mut u8 {
    let layout = Layout::from_size_align(size, 8).unwrap();
    unsafe { KernelAllocator.alloc(layout) }
}

/// Allocate memory with specific layout
pub unsafe fn alloc_aligned(layout: Layout) -> *mut u8 {
    unsafe { KernelAllocator.alloc(layout) }
}

/// Free memory
pub unsafe fn free(ptr: *mut u8) {
    let layout = Layout::from_size_align(1, 1).unwrap();
    unsafe { KernelAllocator.dealloc(ptr, layout) };
}

/// Reallocate memory (Default alignment = 8)
pub unsafe fn realloc(ptr: *mut u8, size: usize) -> *mut u8 {
    let layout = Layout::from_size_align(1, 8).unwrap();
    unsafe { KernelAllocator.realloc(ptr, layout, size) }
}

/// Reallocate memory with alignment
pub unsafe fn realloc_aligned(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    unsafe { KernelAllocator.realloc(ptr, layout, new_size) }
}
