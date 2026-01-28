use core::arch::asm;

/// Write a byte to the specified port
pub unsafe fn outb(port: u16, val: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
    }
}

/// Read a byte from the specified port
pub unsafe fn inb(port: u16) -> u8 {
    let ret: u8;
    unsafe {
        asm!("in al, dx", out("al") ret, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    ret
}

/// Wait a very small amount of time (1-4 microseconds)
/// Useful for I/O port delays
pub unsafe fn io_wait() {
    unsafe {
        // Writing to port 0x80 is commonly used for a small delay
        outb(0x80, 0);
    }
}
