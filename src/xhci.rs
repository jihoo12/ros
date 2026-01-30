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

#[repr(C)]
pub struct EventRingSegmentTableEntry {
    pub base_address: u64,
    pub size: u16,
    pub reserved: [u16; 3],
}

pub const TRB_COMMAND_COMPLETION_EVENT: u8 = 33;
pub const TRB_PORT_STATUS_CHANGE_EVENT: u8 = 34;

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default)]
pub struct Trb {
    pub param: u64,
    pub status: u32,
    pub control: u32,
}

impl Trb {
    pub fn trb_type(&self) -> u8 {
        ((self.control >> 10) & 0x3F) as u8
    }

    pub fn cycle_bit(&self) -> bool {
        (self.control & 1) != 0
    }
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
    pub event_ring_base: *mut Trb,
    pub event_ring_size: usize,
    pub event_ring_dequeue_index: usize,
    pub event_ring_cycle_bit: bool,
    pub max_ports: u8,
}

static mut XHCI_CTX: Option<XhciContext> = None;

#[repr(align(4096))]
struct AlignedPage([u8; 4096]);

static mut COMMAND_RING_BUFFER: AlignedPage = AlignedPage([0; 4096]);
static mut DCBAA_BUFFER: AlignedPage = AlignedPage([0; 4096]);
static mut EVENT_RING_SEGMENT_TABLE: AlignedPage = AlignedPage([0; 4096]);
static mut EVENT_RING_BUFFER: AlignedPage = AlignedPage([0; 4096]);

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

    // 5. DCBAA Setup
    println!("xHCI: Setting up DCBAA...");
    let dcbaa_base = core::ptr::addr_of_mut!(DCBAA_BUFFER).cast::<u64>();
    unsafe { core::ptr::write_bytes(dcbaa_base, 0, 4096) };
    unsafe { write_volatile(&mut (*op).dcbaap, dcbaa_base as u64) };

    // 6. Event Ring Setup (Interrupter 0)
    println!("xHCI: Setting up event ring...");
    let erst_base =
        core::ptr::addr_of_mut!(EVENT_RING_SEGMENT_TABLE).cast::<EventRingSegmentTableEntry>();
    let event_ring_base = core::ptr::addr_of_mut!(EVENT_RING_BUFFER).cast::<u8>();

    unsafe {
        core::ptr::write_bytes(erst_base, 0, 4096);
        core::ptr::write_bytes(event_ring_base, 0, 4096);
    }

    let erst_entry = unsafe { &mut *erst_base };
    erst_entry.base_address = event_ring_base as u64;
    erst_entry.size = (4096 / 16) as u16; // 16 bytes per TRB

    let ir0 = unsafe { &mut (*rt).ir[0] };
    unsafe {
        write_volatile(&mut ir0.erstsz, 1);
        write_volatile(&mut ir0.erstba, erst_base as u64);
        write_volatile(&mut ir0.erdp, event_ring_base as u64);
    }

    // 7. Start Controller
    println!("xHCI: Starting controller...");
    let mut usbcmd = unsafe { read_volatile(&(*op).usbcmd) };
    usbcmd |= 1; // RS: Run/Stop = 1
    unsafe { write_volatile(&mut (*op).usbcmd, usbcmd) };

    while (unsafe { read_volatile(&(*op).usbsts) } & 1) != 0 {
        // Wait for HCH (Host Controller Halted) to become 0
        core::hint::spin_loop();
        break; // In QEMU it might be immediate or we need to be careful with loops
    }
    println!("xHCI: Controller started");

    let max_ports = (hcsparams1 >> 24) as u8;
    println!("xHCI: Max ports: {}", max_ports);

    unsafe {
        XHCI_CTX = Some(XhciContext {
            cap,
            op,
            rt,
            cmd_ring,
            event_ring_base: event_ring_base as *mut Trb,
            event_ring_size: (4096 / 16),
            event_ring_dequeue_index: 0,
            event_ring_cycle_bit: true,
            max_ports,
        });
    }

    unsafe { poll_ports() };
}

pub unsafe fn poll_ports() {
    let ctx = unsafe {
        (*core::ptr::addr_of!(XHCI_CTX))
            .as_ref()
            .expect("xHCI not initialized")
    };
    let op = ctx.op;
    let max_ports = ctx.max_ports;

    println!("xHCI: Polling {} ports...", max_ports);

    for i in 0..max_ports {
        let portsc_ptr = (op as usize + 0x400 + (i as usize * 0x10)) as *mut u32;
        let portsc = unsafe { read_volatile(portsc_ptr) };
        if (portsc & 1) != 0 {
            println!("xHCI: Device detected on port {}", i + 1);
        }
    }
}

pub unsafe fn process_events() {
    let ctx = unsafe {
        (*core::ptr::addr_of_mut!(XHCI_CTX))
            .as_mut()
            .expect("xHCI not initialized")
    };
    let rt = ctx.rt;
    let ir0 = unsafe { &mut (*rt).ir[0] };

    loop {
        let trb_ptr = unsafe { ctx.event_ring_base.add(ctx.event_ring_dequeue_index) };
        let trb = unsafe { read_volatile(trb_ptr) };

        if trb.cycle_bit() != ctx.event_ring_cycle_bit {
            break;
        }

        let trb_type = trb.trb_type();
        if trb_type == TRB_PORT_STATUS_CHANGE_EVENT {
            let port_id = ((trb.param >> 24) & 0xFF) as u8;
            println!("xHCI: Event: Port Status Change on port {}", port_id);
        } else if trb_type == TRB_COMMAND_COMPLETION_EVENT {
            println!("xHCI: Event: Command Completion");
        } else {
            println!("xHCI: Event: Unknown TRB type {}", trb_type);
        }

        ctx.event_ring_dequeue_index += 1;
        if ctx.event_ring_dequeue_index >= ctx.event_ring_size {
            ctx.event_ring_dequeue_index = 0;
            ctx.event_ring_cycle_bit = !ctx.event_ring_cycle_bit;
        }

        let dequeue_ptr =
            unsafe { (ctx.event_ring_base.add(ctx.event_ring_dequeue_index) as u64) | 0x8 };
        unsafe { write_volatile(&mut ir0.erdp, dequeue_ptr) };
    }
}
