use crate::io::{inb, outb, io_wait};

// PIC1 is Master, PIC2 is Slave
const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const PIC_EOI: u8 = 0x20;

// Initialization Command Words
const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;

pub unsafe fn init() {
    unsafe {
        // Remap PIC
        // We want Master to start at 32 (0x20) and Slave at 40 (0x28)
        
        // Save masks
        let _a1 = inb(PIC1_DATA);
        let _a2 = inb(PIC2_DATA);
        
        io_wait();
        
        // ICW1: Init
        outb(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
        io_wait();
        outb(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);
        io_wait();
        
        // ICW2: Vector offsets
        outb(PIC1_DATA, 0x20); // Master starts at 32
        io_wait();
        outb(PIC2_DATA, 0x28); // Slave starts at 40
        io_wait();
        
        // ICW3: Cascading
        outb(PIC1_DATA, 4); // Tell Master that Slave is at IRQ2 (0000 0100)
        io_wait();
        outb(PIC2_DATA, 2); // Tell Slave its cascade identity (0000 0010)
        io_wait();
        
        // ICW4: Mode (8086)
        outb(PIC1_DATA, ICW4_8086);
        io_wait();
        outb(PIC2_DATA, ICW4_8086);
        io_wait();
        
        // Restore masks (or set new ones)
        // For now, let's unmask only Keyboard (IRQ 1) and Timer (IRQ 0)
        // 0 = Unmasked (Enabled), 1 = Masked (Disabled)
        // Mask all initially, then we enable specifically
        outb(PIC1_DATA, 0xFD); // 1111 1101 -> Unmask IRQ 1 (Keyboard) only. Timer masked for now.
        outb(PIC2_DATA, 0xFF);
    }
}

pub unsafe fn notify_eoi(irq: u8) {
    unsafe {
        if irq >= 8 {
            outb(PIC2_COMMAND, PIC_EOI);
        }
        outb(PIC1_COMMAND, PIC_EOI);
    }
}
