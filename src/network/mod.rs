mod driver;
mod e1000;

use crate::memory::{FrameAllocator, PageTable, PAGE_CACHE_DISABLE, PAGE_PRESENT, PAGE_WRITABLE};
use crate::pci::{self, PciDevice};
use crate::println;
use core::ptr::{addr_of, addr_of_mut};
pub use driver::NetworkDriver;

static mut ACTIVE_NIC: Option<driver::Nic> = None;

pub fn is_ready() -> bool {
    unsafe { (*addr_of!(ACTIVE_NIC)).is_some() }
}

/// Probe PCI device, map MMIO/DMA, and bring up the NIC.
pub unsafe fn init(
    pml4: &mut PageTable,
    allocator: &mut FrameAllocator,
    device: PciDevice,
) {
    let Some(mut nic) = driver::Nic::probe(&device) else {
        println!(
            "network: unsupported NIC {:#04x}:{:#04x}",
            device.vendor_id, device.device_id
        );
        return;
    };

    let bar = pci::mmio_bar0(&device);
    let mmio_flags = PAGE_WRITABLE | PAGE_PRESENT | PAGE_CACHE_DISABLE;
    let pages = (nic.mmio_size() + 4095) / 4096;

    println!("network: probing {} at MMIO {:#x}", nic.name(), bar);

    for i in 0..pages {
        let phys = bar + i * 4096;
        crate::memory::map_page(pml4, phys, phys, mmio_flags, allocator);
    }

    nic.map_dma_buffers(pml4, allocator);
    nic.init(device);

    let name = nic.name();
    *addr_of_mut!(ACTIVE_NIC) = Some(nic);
    println!("network: {} ready", name);
}

/// Send a raw Ethernet frame through the active driver.
pub unsafe fn transmit(data: &[u8]) -> bool {
    match &mut *addr_of_mut!(ACTIVE_NIC) {
        Some(nic) => nic.transmit(data),
        None => false,
    }
}

/// Poll the active driver for one received frame.
pub unsafe fn poll_rx(out: &mut [u8]) -> usize {
    match &mut *addr_of_mut!(ACTIVE_NIC) {
        Some(nic) => nic.poll_rx(out),
        None => 0,
    }
}
