use crate::mem::*;
use page_table_entry::{aarch64::A64PTE, GenericPTE, MappingFlags};

/// Number of physical memory regions.
pub(crate) fn memory_regions_num() -> usize {
    common_memory_regions_num() + 1
}

/// Returns the physical memory region at the given index, or [`None`] if the
/// index is out of bounds.
pub(crate) fn memory_region_at(idx: usize) -> Option<MemRegion> {
    use core::cmp::Ordering;
    match idx.cmp(&common_memory_regions_num()) {
        Ordering::Less => common_memory_region_at(idx),
        Ordering::Equal => {
            // free memory
            extern "C" {
                fn eguest();
            }
            let start = virt_to_phys((eguest as usize).into()).align_up_4k();
            let end = PhysAddr::from(axconfig::PHYS_MEMORY_END).align_down_4k();
            Some(MemRegion {
                paddr: start,
                size: end.as_usize() - start.as_usize(),
                flags: MemRegionFlags::FREE | MemRegionFlags::READ | MemRegionFlags::WRITE,
                name: "free memory",
            })
        }
        Ordering::Greater => None,
    }
}

const BOOT_MAP_SHIFT: usize = 30; // 1GB
const BOOT_MAP_SIZE: usize = 1 << BOOT_MAP_SHIFT; // 1GB


// rk3588 由于kernel在低地址，device在高地址 所以直接从kernel往后map
#[cfg(feature = "platform-rk3588-aarch64")]
pub(crate) unsafe fn init_boot_page_table( boot_pt_l0: &mut [A64PTE; 512], boot_pt_l1: &mut [A64PTE; 512],) 
{
    extern "C" {
        fn skernel();
    }

    let aligned_address = (skernel as usize) & !(BOOT_MAP_SIZE - 1);
    let l1_index = (skernel as usize) >> BOOT_MAP_SHIFT;

    // 0x0000_0000_0000 ~ 0x0080_0000_0000, table  0-2G
    boot_pt_l0[0] = A64PTE::new_table(PhysAddr::from(boot_pt_l1.as_ptr() as usize));
    // 1G block, kernel img
    boot_pt_l1[l1_index] = A64PTE::new_page(
        PhysAddr::from(aligned_address),
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE,
        true,
    );

    // 0x0000_4000_0000..0x0000_8000_0000, 1G block, normal memory
    boot_pt_l1[1] = A64PTE::new_page(
        PhysAddr::from(aligned_address+0x4000_0000),
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE,
        true,
    );
}


#[cfg(feature = "platform-qemu-virt-aarch64")]
pub(crate) unsafe fn init_boot_page_table( boot_pt_l0: &mut [A64PTE; 512], boot_pt_l1: &mut [A64PTE; 512],) 
{
    // 0x0000_0000_0000 ~ 0x0080_0000_0000, table
    boot_pt_l0[0] = A64PTE::new_table(PhysAddr::from(boot_pt_l1.as_ptr() as usize));
    // 0x0000_0000_0000..0x0000_4000_0000, 1G block, device memory
    boot_pt_l1[0] = A64PTE::new_page(
        PhysAddr::from(0),
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::DEVICE,
        true,
    );
    // 0x0000_4000_0000..0x0000_8000_0000, 1G block, normal memory
    boot_pt_l1[1] = A64PTE::new_page(
        PhysAddr::from(0x4000_0000),
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE,
        true,
    );
}



/*
 * *************  rk3588 *************
 * ***********************************
 * 
 * 0000_0040_0000  - 0000_0049_e000   kernel real
 * 0000_0040_0000  - 0000_4040_0000   kernel map   
 *
 * 0000_fe60_0000  - gicd
 * 0000_fe68_0000  - gicr
 * 
 * user data 
 * 0000_00cd_a000  -     vm0 dtb
 * 0000_00dd_a000  -     vm0 entry
 */