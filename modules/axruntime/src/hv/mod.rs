use axalloc::global_allocator;
use axhal::mem::{PAGE_SIZE_4K, phys_to_virt, virt_to_phys};
use hypercraft::{HostPhysAddr, HostVirtAddr, HyperCraftHal, HyperResult, VCpu};


#[path = "aarch64_kernel/mod.rs"]
pub mod kernel;

pub use kernel::{
    VM_ARRAY, VM_MAX_NUM, 
    is_vcpu_init_ok, is_vcpu_primary_ok, get_vm, init_vm_vcpu, 
    init_vm_emu_device, init_vm_passthrough_device, add_vm, add_vm_vcpu, print_vm, run_vm_vcpu, secondary_main_hv
};


/// An empty struct to implementate of `HyperCraftHal`
#[derive(Clone, Debug)]
pub struct HyperCraftHalImpl;


impl HyperCraftHal for HyperCraftHalImpl {
    fn alloc_pages(num_pages: usize) -> Option<hypercraft::HostVirtAddr> {
        global_allocator()
            .alloc_pages(num_pages, PAGE_SIZE_4K)
            .map(|pa| pa as HostVirtAddr)
            .ok()
    }

    fn dealloc_pages(pa: HostVirtAddr, num_pages: usize) {
        global_allocator().dealloc_pages(pa as usize, num_pages);
    }
    
}
