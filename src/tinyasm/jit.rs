use alloc::alloc::{alloc, dealloc};
use alloc::format;
use alloc::string::String;

use core::alloc::Layout;
use core::ptr;

pub struct JitMemory {
    addr: *mut u8,
    size: usize,
    layout: Layout,
}

impl JitMemory {
    pub fn new(size: usize) -> Result<Self, String> {
        // Allocate page-aligned memory
        let layout =
            Layout::from_size_align(size, 4096).map_err(|_| String::from("Invalid layout"))?;

        let addr = unsafe { alloc(layout) };
        if addr.is_null() {
            return Err(String::from("Failed to allocate memory"));
        }

        Ok(JitMemory { addr, size, layout })
    }

    pub fn write(&mut self, code: &[u8]) -> Result<(), String> {
        if code.len() > self.size {
            return Err(format!(
                "Code size {} exceeds allocated memory size {}",
                code.len(),
                self.size
            ));
        }

        unsafe {
            ptr::copy_nonoverlapping(code.as_ptr(), self.addr, code.len());
        }
        Ok(())
    }

    pub fn make_executable(&self) -> Result<(), String> {
        // In this simple kernel, we assume heap memory is executable by default.
        // If we implement NX bit later, we will need a syscall to change protection.
        Ok(())
    }

    /// Casts the memory to a function pointer `fn() -> u64`.
    ///
    /// # Safety
    /// Caller must ensure that the memory contains valid machine code for this signature.
    pub unsafe fn as_fn_u64(&self) -> extern "C" fn() -> u64 {
        unsafe { core::mem::transmute(self.addr) }
    }
}

impl Drop for JitMemory {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.addr, self.layout);
        }
    }
}
