use crate::platform::aarch64_common::gic::*;

use super::declare_enum_with_handler;

declare_enum_with_handler! {
    pub enum IpiType [IPI_HANDLER_LIST => fn(IpiMessage)] {
        Power => super::guest_psci::psci_ipi_handler,
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Copy, Clone)]
pub enum PowerEvent {
    PsciIpiCpuOn,
    PsciIpiCpuOff,
    PsciIpiCpuReset,
}

#[derive(Clone)]
pub struct IpiPowerMessage {
    pub src: usize,
    pub event: PowerEvent,
    pub entry: usize,
    pub context: usize,
}

pub struct IpiMessage {
    pub ipi_type: IpiType,
    pub ipi_message: IpiInnerMsg,
}

#[derive(Clone)]
pub enum IpiInnerMsg {
    // IpiTPower
    Power(IpiPowerMessage),
}

pub fn ipi_send_msg(target_id: usize, ipi_type: IpiType, ipi_message: IpiInnerMsg) -> bool {
    let msg = IpiMessage { ipi_type, ipi_message };
    ipi_send(target_id, msg)
}

fn ipi_send(target_id: usize, msg: IpiMessage) -> bool {
    // CPU_IF_LIST[target_id].lock().push(msg);
    interrupt_cpu_ipi_send(target_id, IPI_IRQ_NUM);

    true
}

fn ipi_pop_message(cpu_id: usize) -> Option<IpiMessage> {
    // let mut cpu_if = CPU_IF_LIST[cpu_id].lock();
    // let msg = cpu_if.pop();
    // drop the lock manully
    // drop(cpu_if);
    // msg
    None
}

fn ipi_irq_handler() {
    //let cpu_id = current_cpu().id;
    let cpu_id = 1;

    while let Some(ipi_msg) = ipi_pop_message(cpu_id) {
        let ipi_type = ipi_msg.ipi_type;

        if let Some(handler) = IPI_HANDLER_LIST.get(ipi_type as usize) {
            handler(ipi_msg);
        } else {
            error!("illegal ipi type {:?}", ipi_type)
        }
    }
}