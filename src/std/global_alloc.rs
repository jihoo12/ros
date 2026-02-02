use super::{sys_alloc, sys_free, sys_realloc};
use core::alloc::{GlobalAlloc, Layout};

pub struct StdAllocator;

unsafe impl GlobalAlloc for StdAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Use kernel's aligned alloc
        unsafe { sys_alloc(layout.size(), layout.align()) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { sys_free(ptr) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe { sys_realloc(ptr, new_size, layout.align()) }
    }
}
