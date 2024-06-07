use crate::{GuestPageTableTrait, HostPageNum, HostPhysAddr, HostVirtAddr, HyperResult, memory::PAGE_SIZE_4K};

/// The interfaces which the underlginh software(kernel or hypervisor) must implement.
pub trait HyperCraftHal: Sized + Clone {
    /// Page size.
    const PAGE_SIZE: usize = PAGE_SIZE_4K;

    /// Allocates a 4K-sized contiguous physical page, returns its physical address.
    fn alloc_page() -> Option<HostVirtAddr> {
        Self::alloc_pages(1)
    }
    /// Deallocates the given physical page.
    fn dealloc_page(va: HostVirtAddr) {
        Self::dealloc_pages(va, 1)
    }
    /// Allocates contiguous pages, returns its physical address.
    fn alloc_pages(num_pages: usize) -> Option<HostVirtAddr>;
    /// Gives back the allocated pages starts from `pa` to the page allocator.
    fn dealloc_pages(va: HostVirtAddr, num_pages: usize);
    // /// VM-Exit handler
    // fn vmexit_handler(vcpu: &mut crate::VCpu<Self>, vm_exit_info: VmExitInfo);

}
