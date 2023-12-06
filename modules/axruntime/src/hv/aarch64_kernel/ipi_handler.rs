use axhal::arch::hv::ipi::*;
use super::current_cpu;
use super::guest_psci::psci_ipi_handler;

pub fn init_ipi() {
    if !ipi_register(IpiType::Power, psci_ipi_handler) {
        panic!("power_arch_init: failed to register ipi IpiTPower");
    }
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
            debug!("!!!!!!!!! this is handler: {:#?}", handler as *const());
            handler(&ipi_msg);
        }
        let mut cpu_int_list = CPU_INT_LIST.lock();
        msg = cpu_int_list[cpu_id].pop();
    }
}
