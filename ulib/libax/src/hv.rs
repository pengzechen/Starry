//! Hypervisor related functions

pub use axhal::mem::{phys_to_virt, virt_to_phys, PhysAddr};
pub use axruntime::GuestPageTable;
pub use axruntime::HyperCraftHalImpl;
pub use hypercraft::GuestPageTableTrait;

pub use hypercraft::HyperError as Error;
pub use hypercraft::HyperResult as Result;
pub use hypercraft::HyperCraftHal;
pub use hypercraft::{PerCpu, VCpu, VmCpus, VM};
#[cfg(not(target_arch = "aarch64"))]
pub use hypercraft::{HyperCallMsg, VmExitInfo, GuestPhysAddr, GuestVirtAddr, HostPhysAddr, HostVirtAddr};
#[cfg(target_arch = "aarch64")]
pub use hypercraft::VcpusArray;
#[cfg(target_arch = "aarch64")]
pub use axruntime::{
    VM_ARRAY, VM_MAX_NUM, 
    is_vcpu_init_ok, is_vcpu_primary_ok, init_vm_vcpu, add_vm, add_vm_vcpu, print_vm, run_vm_vcpu
};
