pub mod global_alloc;

use core::arch::asm;

// Syscalls
pub unsafe fn syscall(
    id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        in("rax") id,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        in("r9") arg6,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

pub unsafe fn sys_alloc(size: usize, align: usize) -> *mut u8 {
    unsafe { syscall(2, size, align, 0, 0, 0, 0) as *mut u8 }
}

pub unsafe fn sys_free(ptr: *mut u8) {
    unsafe { syscall(3, ptr as usize, 0, 0, 0, 0, 0) };
}

pub unsafe fn sys_realloc(ptr: *mut u8, size: usize, align: usize) -> *mut u8 {
    unsafe { syscall(13, ptr as usize, size, align, 0, 0, 0) as *mut u8 }
}
