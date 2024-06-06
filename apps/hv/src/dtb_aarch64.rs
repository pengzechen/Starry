use arrayvec::ArrayVec;
use fdt::Fdt;

use core::alloc::Layout;
#[derive(Clone, Debug)]
pub struct Device {
    pub base_address: usize,
    pub size: usize,
}

#[derive(Clone, Debug, Default)]
pub struct MachineMeta {
    pub physical_memory_offset: usize,
    pub physical_memory_size: usize,

    pub virtio: ArrayVec<Device, 32>,

    pub pl011: Option<Device>,
    pub pl031: Option<Device>,
    pub pl061: Option<Device>,

    pub intc: ArrayVec<Device, 3>,

    pub pcie: Option<Device>,

    pub flash: ArrayVec<Device, 2>,
}

impl MachineMeta {
    pub fn parse(dtb: usize) -> Self {
        // debug!("parse 30");
        // let layout = Layout::from_size_align(1024, 4096).unwrap();
        // let area_base: *mut u8 = unsafe { alloc::alloc::alloc_zeroed(layout)};
        // unsafe {*area_base = 1u8;}
        // info!("layout size:{} lay out data {}",layout.size(),layout.);
        // unsafe {debug!("{:?}",*area_base);}
        let fdt = unsafe { Fdt::from_ptr(dtb as *const u8) }.unwrap();
        // debug!("parse 32");
        let memory = fdt.memory();
        let mut meta = MachineMeta::default();
        // debug!("parse 35");
        for region in memory.regions() {
            meta.physical_memory_offset = region.starting_address as usize;
            meta.physical_memory_size = region.size.unwrap();
        }
        // probe virtio mmio device
        for node in fdt.find_all_nodes("/virtio_mmio") {
            if let Some(reg) = node.reg().and_then(|mut reg| reg.next()) {
                let paddr = reg.starting_address as usize;
                let size = reg.size.unwrap();
                // debug!("virtio mmio addr: {:#x}, size: {:#x}", paddr, size);
                meta.virtio.push(Device {
                    base_address: paddr,
                    size,
                })
            }
        }
        meta.virtio.sort_unstable_by_key(|v| v.base_address);

        // probe uart device
        for node in fdt.find_all_nodes("/pl011") {
            if let Some(reg) = node.reg().and_then(|mut reg| reg.next()) {
                let base_addr = reg.starting_address as usize;
                let size = reg.size.unwrap();
                // debug!("pl011 addr: {:#x}, size: {:#x}", base_addr, size);
                meta.pl011 = Some(Device {
                    base_address: base_addr,
                    size,
                });
            }
        }
        for node in fdt.find_all_nodes("/pl031") {
            if let Some(reg) = node.reg().and_then(|mut reg| reg.next()) {
                let base_addr = reg.starting_address as usize;
                let size = reg.size.unwrap();
                // debug!("pl031 addr: {:#x}, size: {:#x}", base_addr, size);
                meta.pl031 = Some(Device {
                    base_address: base_addr,
                    size,
                });
            }
        }
        for node in fdt.find_all_nodes("/pl061") {
            if let Some(reg) = node.reg().and_then(|mut reg| reg.next()) {
                let base_addr = reg.starting_address as usize;
                let size = reg.size.unwrap();
                // debug!("pl061 addr: {:#x}, size: {:#x}", base_addr, size);
                meta.pl061 = Some(Device {
                    base_address: base_addr,
                    size,
                });
            }
        }
        // probe intc(gicc, gicd, gich)
        for node in fdt.find_all_nodes("/intc") {
            let regions = node.reg().unwrap();
            for region in regions {
                let paddr = region. starting_address as usize;
                let size = region.size.unwrap();
                // debug!("intc addr: {:#x}, size: {:#x}", paddr, size);
                meta.intc.push(Device {
                    base_address: paddr,
                    size,
                })
            }
        }
        for node in fdt.find_all_nodes("/intc/v2m") {
            let regions = node.reg().unwrap();
            for region in regions {
                let paddr = region. starting_address as usize;
                let size = region.size.unwrap();
                // debug!("intc addr: {:#x}, size: {:#x}", paddr, size);
                meta.intc.push(Device {
                    base_address: paddr,
                    size,
                })
            }
        }

        meta.intc.sort_unstable_by_key(|v| v.base_address);

        for node in fdt.find_all_nodes("/pcie") {
            if let Some(reg) = node.reg().and_then(|mut reg| reg.next()) {
                let base_addr = reg.starting_address as usize;
                let size = reg.size.unwrap();
                // debug!("pcie addr: {:#x}, size: {:#x}", base_addr, size);
                meta.pcie = Some(Device {
                    base_address: base_addr,
                    size,
                });
            }
        }

        for node in fdt.find_all_nodes("/flash") {
            let regions = node.reg().unwrap();
            for region in regions {
                let paddr = region. starting_address as usize;
                let size = region.size.unwrap();
                // debug!("flash addr: {:#x}, size: {:#x}", paddr, size);
                meta.flash.push(Device {
                    base_address: paddr,
                    size,
                })
            }
        }

        meta
    }
}
