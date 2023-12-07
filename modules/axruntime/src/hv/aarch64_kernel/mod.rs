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

use axhal::{gicc_get_current_irq, deactivate_irq};

/// get current cpu
pub fn current_cpu() -> &'static mut PerCpu<HyperCraftHalImpl> {
    let cpu_id = axhal::cpu::this_cpu_id();
    PerCpu::<HyperCraftHalImpl>::ptr_for_cpu(cpu_id)
}

pub fn secondary_main_hv(cpu_id: usize) {
    info!("Hello World from cpu {}", cpu_id);

    let (irq, src) = gicc_get_current_irq();
    deactivate_irq(irq);
    debug!("after wfi in secondary CPU {} irq id {} src {}", cpu_id, irq, src);

    while !is_vcpu_primary_ok() {
        core::hint::spin_loop();
    }
    PerCpu::<HyperCraftHalImpl>::setup_this_cpu(cpu_id);
    let percpu = PerCpu::<HyperCraftHalImpl>::this_cpu();
    let vcpu = percpu.create_vcpu(0, 1).unwrap();
    percpu.set_active_vcpu(Some(vcpu.clone()));
    add_vm_vcpu(0, vcpu);
    while !is_vcpu_init_ok() {
        core::hint::spin_loop();
    }
    info!("vcpu {} init ok", cpu_id);
    
    debug!("is irq enabled: {}", axhal::arch::irqs_enabled());
    axhal::trap::handle_irq_extern_hv(irq, cpu_id);
}
