use axhal::arch::hv::ipi::*;
use super::current_cpu;

pub fn handle_virtual_interrupt(irq_num: usize, src: usize) {
    debug!("src {:#x} id{:#x} virtual interrupt not implement yet", src, irq_num);
    /*
    if int_id >= 16 && int_id < 32 {
        if let Some(vcpu) = &current_cpu().active_vcpu {
            if let Some(active_vm) = vcpu.vm() {
                if active_vm.has_interrupt(int_id) {
                    interrupt_vm_inject(active_vm, vcpu.clone(), int_id, src);
                } 
            }
        }
    }

    for vcpu in current_cpu().vcpu_array.iter() {
        if let Some(vcpu) = vcpu {
            match vcpu.vm() {
                Some(vm) => {
                    if vm.has_interrupt(int_id) {
                        if vcpu.state() as usize == VcpuState::VcpuInv as usize {
                            return true;
                        }

                        interrupt_vm_inject(vm, vcpu.clone(), int_id, src);
                        return false;
                    }
                }
                None => {}
            }
        }
    }
     */
    debug!(
        "interrupt_handler: core {} receive unsupported int {}",
        current_cpu().cpu_id,
        irq_num
    );
}