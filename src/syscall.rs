use core::arch::asm;
use crate::gdt;

// MSR Constants
const MSR_EFER: u32 = 0xC0000080;
const MSR_STAR: u32 = 0xC0000081;
const MSR_LSTAR: u32 = 0xC0000082;
const MSR_SFMASK: u32 = 0xC0000084;
const MSR_KERNEL_GS_BASE: u32 = 0xC0000102;

// EFER bits
const EFER_SCE: u64 = 1; // System Call Extensions

#[repr(C)]
pub struct KernelGsBase {
    pub kernel_stack: u64,
    pub user_stack: u64,
    pub scratch: u64, // Scratch space if needed
}

static mut KERNEL_GS_BASE: KernelGsBase = KernelGsBase {
    kernel_stack: 0,
    user_stack: 0,
    scratch: 0,
};

// We need a kernel stack for syscalls.
// allocating 16KB stack
static mut SYSCALL_STACK: [u8; 16384] = [0; 16384];

pub unsafe fn init() {
    unsafe {
        // 1. Enable SCE in EFER
        let efer = rdmsr(MSR_EFER);
        wrmsr(MSR_EFER, efer | EFER_SCE);

        // 2. Setup STAR
        // Kernel Code is 0x08.
        // User Code is 0x20.
        
        let star_val: u64 = ((0x0010 as u64) << 48) | ((gdt::KERNEL_CODE_SEL as u64) << 32);
        wrmsr(MSR_STAR, star_val);

        // 3. Setup LSTAR (Target RIP)
        let handler_addr = syscall_handler as u64;
        wrmsr(MSR_LSTAR, handler_addr);

        // 4. Setup SFMASK (RFLAGS mask)
        // Mask interrupts (bit 9, 0x200) so cli is automatic on entry
        wrmsr(MSR_SFMASK, 0x200);

        // 5. Setup Kernel Stack via GS Base
        // Use raw pointers to avoid creating references to static muts (which is error in Rust 2024)
        let stack_ptr = core::ptr::addr_of_mut!(SYSCALL_STACK) as *mut u8;
        // Actually SYSCALL_STACK.len() might borrow. use 16384 directly.
        let stack_end = stack_ptr.add(16384) as u64;
    
        let kgs_base = core::ptr::addr_of_mut!(KERNEL_GS_BASE);
        (*kgs_base).kernel_stack = stack_end;
        
        wrmsr(MSR_KERNEL_GS_BASE, kgs_base as u64);
    }
}

unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high, options(nomem, nostack, preserves_flags));
    }
    ((high as u64) << 32) | (low as u64)
}

unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nomem, nostack, preserves_flags));
    }
}

#[unsafe(naked)]
unsafe extern "C" fn syscall_handler() {
    core::arch::naked_asm!(
        "swapgs",
        "mov gs:[8], rsp", 
        "mov rsp, gs:[0]",
        "push r11", 
        "push rcx", 
        "push rbp",
        "push rbx",
        "push r12", 
        "push r13", 
        "push r14", 
        "push r15", 
        
        "push r9", // Save old R9 (Arg 6)
        
        "mov r9, r8",  // Arg 5
        "mov r8, r10", // Arg 4
        "mov rcx, rdx", // Arg 3
        "mov rdx, rsi", // Arg 2
        "mov rsi, rdi", // Arg 1
        "mov rdi, rax", // Syscall ID
        
        "pop rax", // Pop old R9 into RAX temporarily
        "push rax", // Push it as 7th argument (on stack)
        
        "call {dispatcher}",
        
        "add rsp, 8", // Pop argument
        
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "pop rcx", 
        "pop r11", 

        "mov rsp, gs:[8]",
        "swapgs",
        "sysretq",
        dispatcher = sym syscall_dispatcher_impl,
    );
}

#[unsafe(no_mangle)]
extern "sysv64" fn syscall_dispatcher_impl(
    id: usize,
    arg1: usize,
    arg2: usize, 
    _arg3: usize, 
    _arg4: usize, 
    _arg5: usize,
    _arg6: usize
) -> usize {
    match id {
        1 => {
             // sys_print(ptr, len)
             sys_print(arg1, arg2);
             0
        }
        2 => {
            // sys_alloc(size)
            sys_alloc(arg1)
        }
        3 => {
            // sys_free(ptr)
            sys_free(arg1);
            0
        }
        4 => {
             // sys_get_key() -> u8 (or 0 if empty)
             sys_get_key()
        }
        _ => {
            // Unknown syscall
            let _ = crate::println!("Unknown syscall: {}", id);
            usize::MAX
        }
    }
}

use core::slice;
use core::str;

fn sys_print(ptr: usize, len: usize) {
    let slice = unsafe { slice::from_raw_parts(ptr as *const u8, len) };
    if let Ok(s) = str::from_utf8(slice) {
        crate::print!("{}", s);
    } else {
        crate::print!("(sys_print: invalid utf8)");
    }
}

fn sys_alloc(size: usize) -> usize {
    unsafe {
        crate::allocator::alloc(size) as usize
    }
}

fn sys_free(ptr: usize) {
    unsafe {
        crate::allocator::free(ptr as *mut u8);
    }
}

fn sys_get_key() -> usize {
    unsafe {
        if let Some(c) = crate::keyboard::pop_key() {
            c as usize
        } else {
            0
        }
    }
}

#[inline(always)]
unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize, arg6: usize) -> usize {
    let ret: usize;
    unsafe {
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
    }
    ret
}
