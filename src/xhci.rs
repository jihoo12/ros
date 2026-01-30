#![allow(dead_code)]
#![allow(unused_variables)]

use crate::pci::PciDevice;
use crate::println;
use core::ptr::{read_volatile, write_volatile};

// xHCI Capability Registers (Offset 0x00 from BAR)
#[repr(C)]
pub struct XhciCapabilityRegisters {
    pub caplength: u8, // Capability Register Length
    pub reserved1: u8,
    pub hciversion: u16, // Interface Version Number
    pub hcsparams1: u32, // Structural Parameters 1
    pub hcsparams2: u32, // Structural Parameters 2
    pub hcsparams3: u32, // Structural Parameters 3
    pub hccparams1: u32, // Capability Parameters 1
    pub dboff: u32,      // Doorbell Offset
    pub rtsoff: u32,     // Runtime Register Space Offset
    pub hccparams2: u32, // Capability Parameters 2
}

// xHCI Operational Registers (Offset CAPLENGTH from BAR)
#[repr(C)]
pub struct XhciOperationalRegisters {
    pub usbcmd: u32,   // USB Command
    pub usbsts: u32,   // USB Status
    pub pagesize: u32, // Page Size
    pub reserved1: [u32; 2],
    pub dnctrl: u32, // Device Notification Control
    pub crcr: u64,   // Command Ring Control Register
    pub reserved2: [u32; 4],
    pub dcbaap: u64, // Device Context Base Address Array Pointer
    pub config: u32, // Configure Register
}

// xHCI Runtime Registers (Offset RTSOFF from BAR)
#[repr(C)]
pub struct XhciRuntimeRegisters {
    pub mfindex: u32, // Microframe Index
    pub reserved1: [u32; 7],
    // Interrupter Register Sets start here (at least 1)
    pub ir: [XhciInterrupterRegisterSet; 1024], // Max 1024 interrupters
}

#[repr(C)]
pub struct XhciInterrupterRegisterSet {
    pub iman: u32,   // Interrupter Management
    pub imod: u32,   // Interrupter Moderation
    pub erstsz: u32, // Event Ring Segment Table Size
    pub reserved: u32,
    pub erstba: u64, // Event Ring Segment Table Base Address
    pub erdp: u64,   // Event Ring Dequeue Pointer
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default)]
pub struct Trb {
    pub param: u64,
    pub status: u32,
    pub control: u32,
}

pub struct CommandRing {
    pub base: *mut Trb,
    pub size: usize,
    pub enqueue_index: usize,
    pub cycle_bit: bool,
}

pub struct XhciContext {
    pub cap: *const XhciCapabilityRegisters,
    pub op: *mut XhciOperationalRegisters,
    pub rt: *mut XhciRuntimeRegisters,
    pub cmd_ring: CommandRing,
}

static mut XHCI_CTX: Option<XhciContext> = None;

#[repr(align(4096))]
struct AlignedPage([u8; 4096]);
static mut COMMAND_RING_BUFFER: AlignedPage = AlignedPage([0; 4096]);

pub unsafe fn init(device: PciDevice) {
    println!("xHCI: Initializing...");

    let mut bar = (device.bar0 as u64) & 0xFFFFFFF0;
    let bar_type = (device.bar0 >> 1) & 0x3;
    if bar_type == 2 {
        bar |= (device.bar1 as u64) << 32;
    }

    let cap = bar as *const XhciCapabilityRegisters;
    let caplength = unsafe { read_volatile(&(*cap).caplength) as usize };
    let op = (bar + caplength as u64) as *mut XhciOperationalRegisters;

    let rtsoff = unsafe { read_volatile(&(*cap).rtsoff) & 0xFFFF_FFE0 };
    let rt = (bar + rtsoff as u64) as *mut XhciRuntimeRegisters;

    println!("xHCI: CAPLENGTH={}, RTSOFF={:#x}", caplength, rtsoff);

    // 1. Reset Controller
    println!("xHCI: Resetting controller...");
    let mut usbcmd = unsafe { read_volatile(&(*op).usbcmd) };
    usbcmd |= 1 << 1; // HCRST: Host Controller Reset
    unsafe { write_volatile(&mut (*op).usbcmd, usbcmd) };

    // Wait for HCRST to become 0
    while (unsafe { read_volatile(&(*op).usbcmd) } & (1 << 1)) != 0 {
        core::hint::spin_loop();
    }
    println!("xHCI: Reset completed");

    // 2. Wait for Controller Not Ready (CNR) to become 0
    println!("xHCI: Waiting for CNR...");
    while (unsafe { read_volatile(&(*op).usbsts) } & (1 << 11)) != 0 {
        core::hint::spin_loop();
    }
    println!("xHCI: Controller ready");

    // 3. Max Device Slots
    let hcsparams1 = unsafe { read_volatile(&(*cap).hcsparams1) };
    let max_slots = hcsparams1 & 0xFF;
    println!("xHCI: Max device slots: {}", max_slots);

    // Enable slots
    let mut config = unsafe { read_volatile(&(*op).config) };
    config &= !0xFF;
    config |= max_slots;
    unsafe { write_volatile(&mut (*op).config, config) };

    // 4. Command Ring Setup
    println!("xHCI: Setting up command ring...");
    let cmd_ring_base = core::ptr::addr_of_mut!(COMMAND_RING_BUFFER).cast::<Trb>();
    let cmd_ring_size = 4096 / core::mem::size_of::<Trb>();

    // Clear buffer
    unsafe { core::ptr::write_bytes(cmd_ring_base, 0, cmd_ring_size) };

    let cmd_ring = CommandRing {
        base: cmd_ring_base,
        size: cmd_ring_size,
        enqueue_index: 0,
        cycle_bit: true,
    };

    let crcr = (cmd_ring_base as u64) | 1; // Bit 0: RCS (Ring Cycle State) = 1
    unsafe { write_volatile(&mut (*op).crcr, crcr) };
    println!(
        "xHCI: Command ring configured at {:#x}",
        cmd_ring_base as u64
    );

    unsafe {
        XHCI_CTX = Some(XhciContext {
            cap,
            op,
            rt,
            cmd_ring,
        });
    }
}
