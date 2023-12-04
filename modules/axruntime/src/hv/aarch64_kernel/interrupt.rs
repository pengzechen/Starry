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

pub fn ipi_irq_handler() {
    debug!("ipi handler");
    
    let cpu_id = current_cpu().cpu_id;
    let mut cpu_if_list = CPU_INT_LIST.lock();
    let mut msg: Option<IpiMessage> = cpu_if_list[cpu_id].pop();
    drop(cpu_if_list);

    while !msg.is_none() {
        let ipi_msg = msg.unwrap();
        let ipi_type = ipi_msg.ipi_type as usize;

        let ipi_handler_list = IPI_HANDLER_LIST.lock();
        let len = ipi_handler_list.len();
        let handler = ipi_handler_list[ipi_type].handler.clone();
        drop(ipi_handler_list);

        if len <= ipi_type {
            debug!("illegal ipi type {}", ipi_type)
        } else {
            // debug!("ipi type is {:#?}", ipi_msg.ipi_type);
            handler(&ipi_msg);
        }
        let mut cpu_if_list = CPU_INT_LIST.lock();
        msg = cpu_if_list[cpu_id].pop();
    }
}