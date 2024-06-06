//! Physical memory management.

use core::fmt;

#[cfg(feature = "hv")]
pub const PHYS_VIRT_OFFSET: usize = axconfig::HV_PHYS_VIRT_OFFSET;
#[cfg(not(feature = "hv"))]
pub const PHYS_VIRT_OFFSET:usize = axconfig::PHYS_VIRT_OFFSET;

#[doc(no_inline)]
pub use memory_addr::{PhysAddr, VirtAddr, PAGE_SIZE_4K};

bitflags::bitflags! {
    /// The flags of a physical memory region.
    pub struct MemRegionFlags: usize {
        /// Readable.
        const READ          = 1 << 0;
        /// Writable.
        const WRITE         = 1 << 1;
        /// Executable.
        const EXECUTE       = 1 << 2;
        /// Device memory. (e.g., MMIO regions)
        const DEVICE        = 1 << 4;
        /// Uncachable memory. (e.g., framebuffer)
        // const UNCACHED      = 1 << 5;
        /// Reserved memory, do not use for allocation.
        const RESERVED      = 1 << 5;
        /// Free memory for allocation.
        const FREE          = 1 << 6;
    }
}

impl fmt::Debug for MemRegionFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

/// A physical memory region.
#[derive(Debug)]
pub struct MemRegion {
    /// The start physical address of the region.
    pub paddr: PhysAddr,
    /// The size in bytes of the region.
    pub size: usize,
    /// The region flags, see [`MemRegionFlags`].
    pub flags: MemRegionFlags,
    /// The region name, used for identification.
    pub name: &'static str,
}

/// Converts a virtual address to a physical address.
///
/// It assumes that there is a linear mapping with the offset
/// [`PHYS_VIRT_OFFSET`], that maps all the physical memory to the virtual
/// space at the address plus the offset. So we have
/// `paddr = vaddr - PHYS_VIRT_OFFSET`.
///
/// [`PHYS_VIRT_OFFSET`]: axconfig::PHYS_VIRT_OFFSET
#[inline]
pub const fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    PhysAddr::from(vaddr.as_usize() - PHYS_VIRT_OFFSET)
}

/// Converts a physical address to a virtual address.
///
/// It assumes that there is a linear mapping with the offset
/// [`PHYS_VIRT_OFFSET`], that maps all the physical memory to the virtual
/// space at the address plus the offset. So we have
/// `vaddr = paddr + PHYS_VIRT_OFFSET`.
///
/// [`PHYS_VIRT_OFFSET`]: axconfig::PHYS_VIRT_OFFSET
#[inline]
pub const fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    VirtAddr::from(paddr.as_usize() + PHYS_VIRT_OFFSET)
}

/// Returns an iterator over all physical memory regions.
pub fn memory_regions() -> impl Iterator<Item = MemRegion> {
    kernel_image_regions().chain(crate::platform::aarch64_common::mem::platform_regions())
}

/// Returns the memory regions of the kernel image (code and data sections).
fn kernel_image_regions() -> impl Iterator<Item = MemRegion> {
    [
        // MemRegion {
        //     paddr: virt_to_phys((stext as usize).into()),
        //     size: etext as usize - stext as usize,
        //     flags: MemRegionFlags::RESERVED | MemRegionFlags::READ | MemRegionFlags::EXECUTE,
        //     name: ".text",
        // },
        // MemRegion {
        //     paddr: virt_to_phys((srodata as usize).into()),
        //     size: erodata as usize - srodata as usize,
        //     flags: MemRegionFlags::RESERVED | MemRegionFlags::READ,
        //     name: ".rodata",
        // },
        // MemRegion {
        //     paddr: virt_to_phys((sdata as usize).into()),
        //     size: edata as usize - sdata as usize,
        //     flags: MemRegionFlags::RESERVED | MemRegionFlags::READ | MemRegionFlags::WRITE,
        //     name: ".data .tdata .tbss .percpu",
        // },
        // MemRegion {
        //     paddr: virt_to_phys((boot_stack as usize).into()),
        //     size: boot_stack_top as usize - boot_stack as usize,
        //     flags: MemRegionFlags::RESERVED | MemRegionFlags::READ | MemRegionFlags::WRITE,
        //     name: "boot stack",
        // },
        // MemRegion {
        //     paddr: virt_to_phys((sbss as usize).into()),
        //     size: ebss as usize - sbss as usize,
        //     flags: MemRegionFlags::RESERVED | MemRegionFlags::READ | MemRegionFlags::WRITE,
        //     name: ".bss",
        // },



        MemRegion {
            paddr: virt_to_phys((stext as usize).into()),
            size: etext as usize - stext as usize,
            flags: MemRegionFlags::RESERVED | MemRegionFlags::READ | MemRegionFlags::EXECUTE,
            name: ".text",
        },
        MemRegion {
            paddr: virt_to_phys((srodata as usize).into()),
            size: erodata as usize - srodata as usize,
            flags: MemRegionFlags::RESERVED | MemRegionFlags::READ,
            name: ".rodata",
        },
        MemRegion {
            paddr: virt_to_phys((sdata as usize).into()),
            size: edata as usize - sdata as usize,
            flags: MemRegionFlags::RESERVED | MemRegionFlags::READ | MemRegionFlags::WRITE,
            name: ".data",
        },
        MemRegion {
            paddr: virt_to_phys((__sguestdata as usize).into()),
            size: __eguestdata as usize - __sguestdata as usize,
            flags: MemRegionFlags::RESERVED | MemRegionFlags::READ  | MemRegionFlags::EXECUTE,
            name: ".guestdata",
        },
        MemRegion {
            paddr: virt_to_phys((percpu_start as usize).into()),
            size: percpu_end as usize - percpu_start as usize,
            flags: MemRegionFlags::RESERVED | MemRegionFlags::READ | MemRegionFlags::WRITE,
            name: ".percpu",
        },
        MemRegion {
            paddr: virt_to_phys((boot_stack as usize).into()),
            size: boot_stack_top as usize - boot_stack as usize,
            flags: MemRegionFlags::RESERVED | MemRegionFlags::READ | MemRegionFlags::WRITE,
            name: "boot stack",
        },
        MemRegion {
            paddr: virt_to_phys((sbss as usize).into()),
            size: ebss as usize - sbss as usize,
            flags: MemRegionFlags::RESERVED | MemRegionFlags::READ | MemRegionFlags::WRITE,
            name: ".bss",
        },
        MemRegion {
            paddr: virt_to_phys((sguest as usize).into()),
            size: eguest as usize - sguest as usize,
            flags: MemRegionFlags::RESERVED | MemRegionFlags::READ | MemRegionFlags::WRITE,
            name: ".guest",
        },
        // MemRegion {
        //     paddr: mmio_regions[i - 6].0.into(),
        //     size: mmio_regions[i - 6].1,
        //     flags: MemRegionFlags::RESERVED
        //         | MemRegionFlags::DEVICE
        //         | MemRegionFlags::READ
        //         | MemRegionFlags::WRITE,
        //     name: "mmio",
        // },
    ]
    .into_iter()
}

/// Returns the default MMIO memory regions (from [`axconfig::MMIO_REGIONS`]).
#[allow(dead_code)]
pub(crate) fn default_mmio_regions() -> impl Iterator<Item = MemRegion> {
    axconfig::MMIO_REGIONS.iter().map(|reg| MemRegion {
        paddr: reg.0.into(),
        size: reg.1,
        flags: MemRegionFlags::RESERVED
            | MemRegionFlags::DEVICE
            | MemRegionFlags::READ
            | MemRegionFlags::WRITE,
        name: "mmio",
    })
}

/// Returns the default free memory regions (kernel image end to physical memory end).
#[allow(dead_code)]
pub(crate) fn default_free_regions() -> impl Iterator<Item = MemRegion> {
    debug!("this is a test default_free_regions");
    let start = virt_to_phys((ekernel as usize).into()).align_up_4k();
    let end = PhysAddr::from(axconfig::PHYS_MEMORY_END).align_down_4k();
    core::iter::once(MemRegion {
        paddr: start,
        size: end.as_usize() - start.as_usize(),
        flags: MemRegionFlags::FREE | MemRegionFlags::READ | MemRegionFlags::WRITE,
        name: "free memory",
    })
}

/// Return the extend free memory regions to prepare for the monolithic_userboot
///
/// extend to [0xffff_ffc0_a000_0000, 0xffff_ffc0_f000_0000)
#[allow(dead_code)]
pub(crate) fn extend_free_regions() -> impl Iterator<Item = MemRegion> {
    let start = virt_to_phys(VirtAddr::from(0xffff_ffc0_a000_0000)).align_up_4k();
    let end: PhysAddr = PhysAddr::from(0x1_a000_0000).align_down_4k();
    core::iter::once(MemRegion {
        paddr: start,
        size: end.as_usize() - start.as_usize(),
        flags: MemRegionFlags::FREE | MemRegionFlags::READ | MemRegionFlags::WRITE,
        name: "extend free memory",
    })
}
/// Fills the `.bss` section with zeros.
#[allow(dead_code)]
pub(crate) fn clear_bss() {
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss();
    fn ebss();
    fn boot_stack();
    fn boot_stack_top();
    fn percpu_start();
    fn percpu_end();

    fn __sguestdata();
    fn __eguestdata();

    fn sguest();
    fn eguest();
    fn skernel();
    fn ekernel();
}
