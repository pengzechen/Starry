extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;

use lazy_init::LazyInit;
use hypercraft::{VM, VCpu, PerCpu};

use crate::{HyperCraftHalImpl, GuestPageTable};

use axhal::cpu::this_cpu_id;
const VCPU_CNT: usize = 2;
static INITED_VCPUS: AtomicUsize = AtomicUsize::new(0);
pub const VM_MAX_NUM: usize = 8;
pub static mut VM_ARRAY: LazyInit<Vec<Option<VM<HyperCraftHalImpl, GuestPageTable>>>> = LazyInit::new();

/// Add vm vcpu by index
pub fn add_vm_vcpu(vm_id: usize, vcpu:VCpu<HyperCraftHalImpl>) {
    if vm_id >= VM_MAX_NUM {
        panic!("vm_id {} out of bound", vm_id);
    }
    unsafe {
        if let Some(vm_option) = VM_ARRAY.get_mut(vm_id) {
            if let Some(vm) = vm_option {
                vm.add_vm_vcpu(vcpu);
            }
        }
    }
    INITED_VCPUS.fetch_add(1, Ordering::Relaxed);
}

/// Init vm vcpu by index
pub fn init_vm_vcpu(vm_id: usize, vcpu_id: usize, entry:usize, x0:usize) {
    if vm_id >= VM_MAX_NUM {
        panic!("vm_id {} out of bound", vm_id);
    }
    unsafe {
        if let Some(vm_option) = VM_ARRAY.get_mut(vm_id) {
            if let Some(vm) = vm_option {
                vm.init_vm_vcpu(vcpu_id, entry, x0);
            }
        }
    } // debug!("finish init_vm_vcpu vm_id:{} vcpu {:?}", vm_id
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
        debug!("current pcpu id: {} vcpu id:{}", this_cpu_id(), vcpu_id);
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