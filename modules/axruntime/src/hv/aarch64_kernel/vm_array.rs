extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;

use lazy_init::LazyInit;
use hypercraft::{VM, VCpu};

use crate::{HyperCraftHalImpl, GuestPageTable};

const VCPU_CNT: usize = 2;
static INITED_VCPUS: AtomicUsize = AtomicUsize::new(0);
pub const VM_MAX_NUM: usize = 8;
pub static mut VM_ARRAY: LazyInit<Vec<Option<VM<HyperCraftHalImpl, GuestPageTable>>>> = LazyInit::new();

/// Get vm by index
pub fn init_vm_vcpu(vm_id: usize, vcpu: VCpu<HyperCraftHalImpl>) {
    if vm_id >= VM_MAX_NUM {
        panic!("vm_id {} out of bound", vm_id);
    }
    unsafe {
        if let Some(vm_option) = VM_ARRAY.get_mut(vm_id) {
            if let Some(vm) = vm_option {
                vm.add_vm_vcpu(vcpu.clone());
                vm.init_vm_vcpu(vcpu.vcpu_id(), 0x7020_0000, 0x7000_0000);
            }
        }
    }
    debug!("finish init_vm_vcpu vm_id:{} vcpu {:?}", vm_id, vcpu);
    INITED_VCPUS.fetch_add(1, Ordering::Relaxed);
}

/// Add vm to vm array
pub fn add_vm(vm_id: usize, vm: VM<HyperCraftHalImpl, GuestPageTable>) {
    if vm_id >= VM_MAX_NUM {
        panic!("vm_id {} out of bound", vm_id);
    }

    unsafe {
        while VM_ARRAY.len() <= vm_id {
            VM_ARRAY.push(None);
        }
    
        VM_ARRAY[vm_id] = Some(vm);
    }

}

/// Print vm info
pub fn print_vm(vm_id: usize) {
    unsafe {
        if let Some(vm_option) = VM_ARRAY.get_mut(vm_id) {
            if let Some(vm) = vm_option {
                debug!("vcpus: {:?}", vm.vcpus)
            }
        }
    }
}

/// Run vm by id
pub fn run_vm_vcpu(vm_id: usize, vcpu_id: usize) {
    unsafe {
        if let Some(vm_option) = VM_ARRAY.get_mut(vm_id) {
            if let Some(vm) = vm_option {
                vm.run(vcpu_id);
            }
        }
    }
}

/// Checks if the initialization of the virtual machine array is successful.
///
/// Returns `true` if the initialization is successful, `false` otherwise.
pub fn is_vcpu_init_ok() -> bool {
    INITED_VCPUS.load(Ordering::Acquire) == VCPU_CNT
}

/// Checks if the primary virtual CPU (vCPU) is in a valid state.
/// 
/// Returns `true` if the primary vCPU is in a valid state, `false` otherwise.
pub fn is_vcpu_primary_ok() -> bool {
    INITED_VCPUS.load(Ordering::Acquire) == 1
}