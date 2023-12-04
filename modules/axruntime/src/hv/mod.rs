use axalloc::global_allocator;
use axhal::mem::{PAGE_SIZE_4K, phys_to_virt, virt_to_phys};
use hypercraft::{HostPhysAddr, HostVirtAddr, HyperCraftHal, HyperResult, VCpu};

#[cfg(target_arch = "x86_64")]
mod vmx;

#[cfg(target_arch = "aarch64")]
pub mod aarch64_kernel;
#[cfg(target_arch = "aarch64")]
pub use aarch64_kernel::{
    VM_ARRAY, VM_MAX_NUM, 
    is_vcpu_init_ok, is_vcpu_primary_ok, init_vm_vcpu, add_vm, add_vm_vcpu, print_vm, run_vm_vcpu
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

    #[cfg(target_arch = "x86_64")]
    fn phys_to_virt(pa: HostPhysAddr) -> HostVirtAddr {
        phys_to_virt(pa.into()).into()
    }

    #[cfg(target_arch = "x86_64")]
    fn virt_to_phys(va: HostVirtAddr) -> HostPhysAddr {
        virt_to_phys(va.into()).into()
    }

    #[cfg(target_arch = "x86_64")]
    fn vmexit_handler(vcpu: &mut VCpu<Self>) -> HyperResult {
        vmx::vmexit_handler(vcpu)
    }

    #[cfg(target_arch = "x86_64")]
    fn current_time_nanos() -> u64 { 
        axhal::time::current_time_nanos()
    }
}
