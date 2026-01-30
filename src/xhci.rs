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

pub const TRB_NOOP_COMMAND: u8 = 23;
pub const TRB_ENABLE_SLOT_COMMAND: u8 = 9;
pub const TRB_ADDRESS_DEVICE_COMMAND: u8 = 11;
pub const TRB_CONFIGURE_ENDPOINT_COMMAND: u8 = 12;

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

    pub fn completion_code(&self) -> u8 {
        (self.status >> 24) as u8
    }

    pub fn slot_id(&self) -> u8 {
        (self.control >> 24) as u8
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SlotContext {
    pub field1: u32,
    pub field2: u32,
    pub field3: u32,
    pub field4: u32,
    pub reserved: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct EndpointContext {
    pub field1: u32,
    pub field2: u32,
    pub tr_dequeue_pointer: u64,
    pub field4: u32,
    pub reserved: [u32; 3],
}

#[repr(C)]
pub struct DeviceContext {
    pub slot: SlotContext,
    pub endpoints: [EndpointContext; 31],
}

#[repr(C)]
pub struct InputContext {
    pub drop_flags: u32,
    pub add_flags: u32,
    pub reserved1: [u32; 5],
    pub field8: u32, // Input Control Context End
    pub device: DeviceContext,
}
pub struct TransferRing {
    pub base: *mut Trb,
    pub size: usize,
    pub enqueue_index: usize,
    pub cycle_bit: bool,
}

impl TransferRing {
    pub fn new(buffer: *mut Trb, size_bytes: usize) -> Self {
        unsafe { core::ptr::write_bytes(buffer, 0, size_bytes) };
        Self {
            base: buffer,
            size: size_bytes / core::mem::size_of::<Trb>(),
            enqueue_index: 0,
            cycle_bit: true,
        }
    }
}

// Global state for command results (simplified for single-task)
static mut LAST_COMPLETION_CODE: u8 = 0;
static mut LAST_SLOT_ID: u8 = 0;

static mut EP0_TRANSFER_RINGS: [Option<TransferRing>; 64] = [const { None }; 64];
#[repr(align(4096))]
struct TransferRingBuffer([Trb; 256]);
static mut EP0_TR_BUFFERS: [TransferRingBuffer; 64] =
    unsafe { core::mem::transmute([0u8; 64 * 4096]) };

pub struct CommandRing {
    pub base: *mut Trb,
    pub size: usize,
    pub enqueue_index: usize,
    pub cycle_bit: bool,
}

impl CommandRing {
    pub unsafe fn enqueue(&mut self, mut trb: Trb, db: *mut u32) {
        let control = trb.control & !1;
        trb.control = control | (if self.cycle_bit { 1 } else { 0 });

        let trb_ptr = unsafe { self.base.add(self.enqueue_index) };
        unsafe { write_volatile(trb_ptr, trb) };

        self.enqueue_index += 1;
        if self.enqueue_index >= self.size {
            self.enqueue_index = 0;
            self.cycle_bit = !self.cycle_bit;
        }

        // Ring Doorbell 0 (Host Controller)
        unsafe { write_volatile(db, 0) };
    }
}

pub struct XhciContext {
    pub cap: *const XhciCapabilityRegisters,
    pub op: *mut XhciOperationalRegisters,
    pub rt: *mut XhciRuntimeRegisters,
    pub db: *mut u32, // Doorbell Array Base
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

// Device Contexts for up to 64 slots. 1024 bytes each.
#[repr(align(4096))]
struct DeviceContextBuffer([DeviceContext; 64]);
static mut DEVICE_CONTEXT_BUFFERS: DeviceContextBuffer = unsafe { core::mem::zeroed() };

// Input Context for command execution (one at a time is fine for now)
static mut INPUT_CONTEXT_BUFFER: AlignedPage = AlignedPage([0; 4096]);

pub unsafe fn init(device: PciDevice) {
    println!("xHCI: Initializing...");

    let mut bar = (device.bar0 as u64) & 0xFFFFFFF0;
    let bar_type = (device.bar0 >> 1) & 0x3;
    if bar_type == 2 {
        bar |= (device.bar1 as u64) << 32;
    }
    println!("xHCI: BAR at {:#x}", bar);

    let cap = bar as *const XhciCapabilityRegisters;
    let caplength = unsafe { read_volatile(&(*cap).caplength) as usize };
    let op = (bar + caplength as u64) as *mut XhciOperationalRegisters;

    let rtsoff = unsafe { read_volatile(&(*cap).rtsoff) & 0xFFFF_FFE0 };
    let rt = (bar + rtsoff as u64) as *mut XhciRuntimeRegisters;

    let dboff = unsafe { read_volatile(&(*cap).dboff) & 0xFFFF_FFFC };
    let db = (bar + dboff as u64) as *mut u32;

    let hccparams1 = unsafe { read_volatile(&(*cap).hccparams1) };
    println!(
        "xHCI: CAPLENGTH={}, RTSOFF={:#x}, DBOFF={:#x}, HCCPARAMS1={:#x}",
        caplength, rtsoff, dboff, hccparams1
    );

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
    println!("xHCI: DCBAA_BUFFER at {:#x}", dcbaa_base as u64);
    unsafe { core::ptr::write_bytes(dcbaa_base, 0, 4096) };
    unsafe { write_volatile(&mut (*op).dcbaap, dcbaa_base as u64) };

    // 6. Event Ring Setup (Interrupter 0)
    println!("xHCI: Setting up event ring...");
    let erst_base =
        core::ptr::addr_of_mut!(EVENT_RING_SEGMENT_TABLE).cast::<EventRingSegmentTableEntry>();
    let event_ring_base = core::ptr::addr_of_mut!(EVENT_RING_BUFFER).cast::<u8>();
    println!(
        "xHCI: ERST at {:#x}, Event Ring at {:#x}",
        erst_base as u64, event_ring_base as u64
    );

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
            db,
            cmd_ring,
            event_ring_base: event_ring_base as *mut Trb,
            event_ring_size: (4096 / 16),
            event_ring_dequeue_index: 0,
            event_ring_cycle_bit: true,
            max_ports,
        });
    }

    unsafe { poll_ports() };
    unsafe { send_noop_command() };
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
            // Check if port is enabled (bit 1). If not, reset it.
            if (portsc & (1 << 1)) == 0 {
                println!("xHCI: Resetting port {}...", i + 1);
                unsafe {
                    let mut p = read_volatile(portsc_ptr);
                    p &= 0x0E01_C0E1; // Preserve read-write bits, clear change bits
                    p |= 1 << 4; // PR = 1
                    write_volatile(portsc_ptr, p);
                }
                // Wait for PRC (bit 21)
                while (unsafe { read_volatile(portsc_ptr) } & (1 << 21)) == 0 {
                    core::hint::spin_loop();
                }
                // Clear PRC (write 1 to bit 21)
                unsafe {
                    let mut p = read_volatile(portsc_ptr);
                    p &= 0x0E01_C0E1;
                    p |= 1 << 21;
                    write_volatile(portsc_ptr, p);
                }
            }

            let portsc_final = unsafe { read_volatile(portsc_ptr) };
            let speed = (portsc_final >> 10) & 0xF;
            println!(
                "xHCI: Device detected on port {} (speed {}, enabled={})",
                i + 1,
                speed,
                (portsc_final >> 1) & 1
            );
            let slot_id = unsafe { enable_slot() };
            if slot_id > 0 {
                unsafe { address_device(slot_id, i + 1, speed) };
            }
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
            let completion_code = trb.completion_code();
            let slot_id = trb.slot_id();
            println!(
                "xHCI: Event: Command Completion. Code={}, SlotID={}, Param={:#x}",
                completion_code, slot_id, trb.param
            );
            unsafe {
                LAST_COMPLETION_CODE = completion_code;
                LAST_SLOT_ID = slot_id;
            }
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

pub unsafe fn send_noop_command() {
    let ctx = unsafe {
        (*core::ptr::addr_of_mut!(XHCI_CTX))
            .as_mut()
            .expect("xHCI not initialized")
    };
    println!("xHCI: Sending No-op command...");
    let mut trb = Trb::default();
    trb.control = (TRB_NOOP_COMMAND as u32) << 10;
    unsafe { ctx.cmd_ring.enqueue(trb, ctx.db) };
}

pub unsafe fn enable_slot() -> u8 {
    let ctx = unsafe {
        (*core::ptr::addr_of_mut!(XHCI_CTX))
            .as_mut()
            .expect("xHCI not initialized")
    };
    println!("xHCI: Sending Enable Slot command...");
    let mut trb = Trb::default();
    trb.control = (TRB_ENABLE_SLOT_COMMAND as u32) << 10;

    unsafe {
        LAST_COMPLETION_CODE = 0;
        LAST_SLOT_ID = 0;
        ctx.cmd_ring.enqueue(trb, ctx.db);
    }

    // Wait for completion (simple poll)
    while unsafe { read_volatile(core::ptr::addr_of!(LAST_COMPLETION_CODE)) } == 0 {
        unsafe { process_events() };
        core::hint::spin_loop();
    }

    let slot_id = unsafe { read_volatile(core::ptr::addr_of!(LAST_SLOT_ID)) };
    println!("xHCI: Assigned Slot ID: {}", slot_id);
    slot_id
}

pub unsafe fn address_device(slot_id: u8, port_id: u8, speed: u32) {
    let ctx = unsafe {
        (*core::ptr::addr_of_mut!(XHCI_CTX))
            .as_mut()
            .expect("xHCI not initialized")
    };

    println!(
        "xHCI: Addressing device in slot {} (port {})...",
        slot_id, port_id
    );

    // 1. Initialize Device Context
    let device_ctx_ptr = unsafe {
        let buffers_ptr = core::ptr::addr_of_mut!(DEVICE_CONTEXT_BUFFERS);
        let device_ptr = core::ptr::addr_of_mut!((*buffers_ptr).0[slot_id as usize - 1]);
        core::ptr::write_bytes(
            device_ptr as *mut u8,
            0,
            core::mem::size_of::<DeviceContext>(),
        );
        device_ptr
    };

    // Set Slot Context
    unsafe {
        (*device_ctx_ptr).slot.field1 = (1 << 27) | (speed << 20); // Context Entries = 1, Speed
        (*device_ctx_ptr).slot.field2 = (port_id as u32) << 16; // Root Hub Port Number
    }

    // 2. Initialize EP0 Transfer Ring
    let tr_buffer = unsafe {
        let ep0_buffers_ptr = core::ptr::addr_of_mut!(EP0_TR_BUFFERS);
        let buffer_ptr = core::ptr::addr_of_mut!((*ep0_buffers_ptr)[slot_id as usize - 1].0);
        buffer_ptr as *mut Trb
    };
    let tr = TransferRing::new(tr_buffer, 4096);
    unsafe {
        let rings_ptr = core::ptr::addr_of_mut!(EP0_TRANSFER_RINGS);
        (*rings_ptr)[slot_id as usize - 1] = Some(tr);
    }

    // Set EP0 Context
    let mps = match speed {
        3 => 64,  // High Speed
        4 => 512, // Super Speed
        2 => 8,   // Low Speed
        _ => 64,  // Full Speed default
    };
    unsafe {
        (*device_ctx_ptr).endpoints[0].field2 = (4 << 3) | (mps << 16) | (3 << 1); // EP Type = Control, MPS, CErr = 3
        (*device_ctx_ptr).endpoints[0].tr_dequeue_pointer = (tr_buffer as u64) | 1; // DCS = 1
    }

    // 3. Set DCBAA entry
    unsafe {
        let dcbaa = ctx.op.read_volatile().dcbaap as *mut u64;
        write_volatile(
            dcbaa.add(slot_id as usize),
            device_ctx_ptr as *const _ as u64,
        );
    }

    // 4. Prepare Input Context
    let input_ctx_ptr = unsafe {
        let ptr = core::ptr::addr_of_mut!(INPUT_CONTEXT_BUFFER) as *mut InputContext;
        core::ptr::write_bytes(ptr as *mut u8, 0, 4096);
        (*ptr).add_flags = 0x3; // Add Slot Context (bit 0) and EP0 Context (bit 1)
        core::ptr::copy_nonoverlapping(
            device_ctx_ptr as *const u8,
            core::ptr::addr_of_mut!((*ptr).device) as *mut u8,
            core::mem::size_of::<DeviceContext>(),
        );
        ptr
    };

    // 5. Send Address Device Command
    let mut trb = Trb::default();
    trb.param = input_ctx_ptr as u64;
    trb.control = (TRB_ADDRESS_DEVICE_COMMAND as u32) << 10 | (slot_id as u32) << 24; // BSR = 0

    unsafe {
        LAST_COMPLETION_CODE = 0;
        ctx.cmd_ring.enqueue(trb, ctx.db);
    }

    // Wait for completion
    while unsafe { read_volatile(core::ptr::addr_of!(LAST_COMPLETION_CODE)) } == 0 {
        unsafe { process_events() };
        core::hint::spin_loop();
    }

    println!("xHCI: Address Device completed with code {}", unsafe {
        read_volatile(core::ptr::addr_of!(LAST_COMPLETION_CODE))
    });
}
