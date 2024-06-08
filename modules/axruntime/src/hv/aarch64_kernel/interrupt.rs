use super::ipi::*;
use super::{current_cpu, active_vm};
use hypercraft::{VM, VCpu};

#[cfg(not(feature = "gic_v3"))]
use super::vgic::{vgic_inject, vgic_set_hw_int};
#[cfg(feature = "gic_v3")]
use super::vgicv3::{vgic_inject, vgic_set_hw_int};

use crate::{HyperCraftHalImpl, GuestPageTable};

// 选择合适的 vcpu 、vm 调用中断注入
// caller trap : handle_irq_hv
pub fn handle_virtual_interrupt(irq_num: usize, src: usize) {
    debug!("src {:#x} id{:#x} virtual interrupt", src, irq_num);

    let vm = active_vm();
    // only one vm and one vcpu, every interrupt is match to this vcpu
    //if irq_num >= 16 && irq_num < 32 {
        let vcpu = current_cpu().get_active_vcpu().unwrap().clone();
        if vm.has_interrupt(irq_num) {
            interrupt_vm_inject(vm, vcpu, irq_num);
        }
        /* 
        if let Some(vcpu) = &current_cpu().active_vcpu {
            if let Some(active_vm) = vcpu.vm() {
                if active_vm.has_interrupt(irq_num) {
                    interrupt_vm_inject(active_vm, vcpu.clone(), irq_num, src);
                } 
            }
        }
        */
    //}

    // todo: there is only one vcpu bind to a pcpu now
    /* 
    for vcpu in current_cpu().vcpu_array.iter() {
        if let Some(vcpu) = vcpu {
            match vcpu.vm() {
                Some(vm) => {
                    if vm.has_interrupt(irq_num) {
                        if vcpu.state() as usize == VcpuState::VcpuInv as usize {
                            return true;
                        }

                        interrupt_vm_inject(vm, vcpu.clone(), irq_num, src);
                        return false;
                    }
                }
                None => {}
            }
        }
    }
    */

    debug!(
        "interrupt_handler: core {} receive virtual int {}",
        current_cpu().cpu_id,
        irq_num
    );
}

// 中断注入函数，调用 vgic.rs : vgic_inject
pub fn interrupt_vm_inject(vm: &mut VM<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, irq_num: usize) {
    debug!("[interrupt_vm_inject] this is interrupt vm inject");
    let vgic = vm.vgic();
    // restore_vcpu_gic(current_cpu().active_vcpu.clone(), vcpu.clone());
    if let Some(cur_vcpu) = current_cpu().get_active_vcpu().clone() {
        if cur_vcpu.vm_id == vcpu.vm_id {
            debug!("[interrupt_vm_inject] before vgic_inject");
            vgic_inject(&*vgic, vcpu, irq_num);
            debug!("[interrupt_vm_inject] after vm {} inject irq {}", vm.vm_id, irq_num);
            return;
        }
    }

    // vcpu.push_int(irq_num);
    // save_vcpu_gic(current_cpu().active_vcpu.clone(), vcpu.clone());
}

// 虚拟机的中断注册函数
// caller init_vm_passthrough_device ， 最终由app调用
pub fn interrupt_vm_register(vm:& mut VM<HyperCraftHalImpl, GuestPageTable>, id: usize) -> bool {
    debug!("interrupt_vm_register id: {:#x}", id);
    vgic_set_hw_int(vm, id);
    vm.set_int_bit_map(id);
    true
}

