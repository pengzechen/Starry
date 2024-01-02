extern crate alloc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use hypercraft::{PerCpu, VCpu, VM};
use hypercraft::arch::emu::EmuDeviceType;
use lazy_init::LazyInit;

use axhal::cpu::this_cpu_id;

use super::emu::emu_register_dev;
use super::emuintc_handler::{emu_intc_handler, emu_intc_init};
use super::emuuart_handler::{emu_uart_handler, emu_uart_init};
use super::interrupt::interrupt_vm_register;
use crate::{GuestPageTable, HyperCraftHalImpl};

const VCPU_CNT: usize = 2;
static INITED_VCPUS: AtomicUsize = AtomicUsize::new(0);
pub const VM_MAX_NUM: usize = 8;
pub static mut VM_ARRAY: LazyInit<Vec<Option<VM<HyperCraftHalImpl, GuestPageTable>>>> =
    LazyInit::new();

/// Add vm vcpu by index
pub fn add_vm_vcpu(vm_id: usize, vcpu: VCpu<HyperCraftHalImpl>) {
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
}

/// Init vm vcpu by index
pub fn init_vm_vcpu(vm_id: usize, vcpu_id: usize, entry: usize, x0: usize) {
    if vm_id >= VM_MAX_NUM {
        panic!("vm_id {} out of bound", vm_id);
    }
    unsafe {
        if let Some(vm_option) = VM_ARRAY.get_mut(vm_id) {
            if let Some(vm) = vm_option {
                // init vcpu
                vm.init_vm_vcpu(vcpu_id, entry, x0);
            }
        }
    }
}

/// Init vm emulated device
pub fn init_vm_emu_device(vm_id: usize) {
    if vm_id >= VM_MAX_NUM {
        panic!("vm_id {} out of bound", vm_id);
    }
    unsafe {
        if let Some(vm_option) = VM_ARRAY.get_mut(vm_id) {
            if let Some(vm) = vm_option {
                // init emu intc
                let idx = 0;
                vm.set_intc_dev_id(idx);
                emu_register_dev(
                    EmuDeviceType::EmuDeviceTGicd,
                    vm.vm_id,
                    idx,
                    0x8000000, // emu_dev.base_ipa,
                    0x1000,    // emu_dev.length,
                    emu_intc_handler,
                );
                emu_intc_init(vm, idx);

                // init emu uart
                let idx = 1;
                emu_register_dev(
                    EmuDeviceType::EmuDeviceTConsole,
                    vm.vm_id,
                    idx,
                    0x9000000, // emu_dev.base_ipa,
                    0x1000,    // emu_dev.length,
                    emu_uart_handler,
                );
                emu_uart_init(vm, idx);
            }
        }
    }
}

/// init vm passthrough device
pub fn init_vm_passthrough_device(vm_id: usize) {
    if vm_id >= VM_MAX_NUM {
        panic!("vm_id {} out of bound", vm_id);
    }
    unsafe {
        if let Some(vm_option) = VM_ARRAY.get_mut(vm_id) {
            if let Some(vm) = vm_option {
                // hard code for qemu vm
                let mut irqs = Vec::new();
                irqs.push(33);  
                irqs.push(27);  // virtual timer
                // irqs.push(30);
                irqs.push(32 + 0x28);
                irqs.push(32 + 0x29);
                irqs.push(0x3e + 0x11);  // what interrupt????
                for irq in irqs {
                    // debug!("this is irq: {:#x}", irq);
                    if !interrupt_vm_register(vm, irq) {
                        warn!("vm{} register irq{} failed", vm_id, irq);
                    }
                    // debug!("after register for vm irq: {:#x}", irq);
                }
            }
        }
    }
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

/// Get vm by id
pub fn get_vm(vm_id: usize) -> Option<&'static mut VM<HyperCraftHalImpl, GuestPageTable>> {
    unsafe {
        if let Some(vm_option) = VM_ARRAY.get_mut(vm_id) {
            if let Some(vm) = vm_option {
                return Some(vm);
            }
        }
    }
    None
}

/// Run vm by id
pub fn run_vm_vcpu(vm_id: usize, vcpu_id: usize) ->! {
    INITED_VCPUS.fetch_add(1, Ordering::Relaxed);
    unsafe {
        debug!("current pcpu id: {} vcpu id:{}", this_cpu_id(), vcpu_id);
        if let Some(vm_option) = VM_ARRAY.get_mut(vm_id) {
            if let Some(vm) = vm_option {
                vm.run(vcpu_id);
            }
        }
    }
    loop{}
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
