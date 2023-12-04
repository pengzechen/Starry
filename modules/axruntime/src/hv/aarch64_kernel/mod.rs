pub mod vm_array;
mod ipi_handler;
mod interrupt;
mod guest_psci;

pub use vm_array::{
    VM_ARRAY, VM_MAX_NUM, 
    is_vcpu_init_ok, is_vcpu_primary_ok, init_vm_vcpu, add_vm, add_vm_vcpu, print_vm, run_vm_vcpu
};

use hypercraft::PerCpu;
use crate::hv::HyperCraftHalImpl;

pub use interrupt::handle_virtual_interrupt;
pub use ipi_handler::{ipi_irq_handler, init_ipi};

/// get current cpu
pub fn current_cpu() -> &'static mut PerCpu<HyperCraftHalImpl> {
    let cpu_id = axhal::cpu::this_cpu_id();
    PerCpu::<HyperCraftHalImpl>::ptr_for_cpu(cpu_id)
}
