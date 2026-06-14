#![no_std]
#![no_main]

#[inline(always)]
unsafe fn syscall0(id: usize) -> usize {
    let ret: usize;
    core::arch::asm!(
        "syscall",
        in("rax") id,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        out("rdi") _,
        out("rsi") _,
        out("rdx") _,
        out("r10") _,
        out("r8") _,
        out("r9") _,
        options(nostack, preserves_flags)
    );
    ret
}

#[inline(always)]
unsafe fn syscall2(id: usize, arg1: usize, arg2: usize) -> usize {
    let ret: usize;
    core::arch::asm!(
        "syscall",
        in("rax") id,
        in("rdi") arg1,
        in("rsi") arg2,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        out("rdx") _,
        out("r10") _,
        out("r8") _,
        out("r9") _,
        options(nostack, preserves_flags)
    );
    ret
}

pub fn print(s: &str) {
    unsafe {
        syscall2(0, s.as_ptr() as usize, s.len());
    }
}

pub fn read_key() -> usize {
    unsafe {
        syscall0(8)
    }
}

pub fn poll_xhci() {
    unsafe {
        syscall0(6);
    }
}

pub fn yield_task() {
    unsafe {
        syscall0(4);
    }
}

pub fn shutdown() -> ! {
    unsafe {
        syscall0(7);
    }
    loop {}
}
