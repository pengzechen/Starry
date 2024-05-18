mod emu;
mod emuintc_handler;
mod emuuart_handler;
mod guest_psci;
mod interrupt;
mod ipi;
mod sync;

#[cfg(not(feature = "gic_v3"))]
mod vgic;
#[cfg(feature = "gic_v3")]
mod vgicv3;

mod vuart;

pub mod vm_array;

pub use vm_array::{
    VM_ARRAY, VM_MAX_NUM,
    add_vm, add_vm_vcpu, get_vm, print_vm,
    init_vm_vcpu, init_vm_emu_device, init_vm_passthrough_device, 
    is_vcpu_init_ok, is_vcpu_primary_ok,
    run_vm_vcpu, 
};

use crate::{HyperCraftHalImpl, GuestPageTable};
use hypercraft::{PerCpu, VM};

pub use emuintc_handler::gic_maintenance_handler;
pub use interrupt::handle_virtual_interrupt;
pub use ipi::{init_ipi, ipi_irq_handler, cpu_int_list_init};

use axhal::{deactivate_irq, gicc_get_current_irq};

/// get current cpu
pub fn current_cpu() -> &'static mut PerCpu<HyperCraftHalImpl> {
    let cpu_id = axhal::cpu::this_cpu_id();
    PerCpu::<HyperCraftHalImpl>::ptr_for_cpu(cpu_id)
}

/// get active vm
pub fn active_vm() -> &'static mut VM<HyperCraftHalImpl, GuestPageTable> {
    let cpu_id = axhal::cpu::this_cpu_id();
    let percpu = PerCpu::<HyperCraftHalImpl>::ptr_for_cpu(cpu_id);
    let vm_id = percpu.get_active_vcpu().unwrap().vm_id;
    match get_vm(vm_id) {
        Some(vm) => vm,
        None => panic!("No active VM found"),
    }
}

pub fn secondary_main_hv(cpu_id: usize) {
    info!("Hello World from cpu {}", cpu_id);

    let (irq, src) = gicc_get_current_irq();
    deactivate_irq(irq);
    debug!(
        "after wfi in secondary CPU {} irq id {} src {}",
        cpu_id, irq, src
    );

    while !is_vcpu_primary_ok() {
        core::hint::spin_loop();
    }
    PerCpu::<HyperCraftHalImpl>::setup_this_cpu(cpu_id).unwrap();
    let percpu = PerCpu::<HyperCraftHalImpl>::this_cpu();
    let vcpu = percpu.create_vcpu(0, 1).unwrap();
    percpu.set_active_vcpu(Some(vcpu.clone()));
    add_vm_vcpu(0, vcpu);
    while !is_vcpu_init_ok() {
        core::hint::spin_loop();
    }
    info!("vcpu {} init ok", cpu_id);

    debug!("is irq enabled: {}", axhal::arch::irqs_enabled());
    let ctx = current_cpu().get_ctx().unwrap();
    axhal::trap::handle_irq_extern_hv(irq, cpu_id, ctx);
}
