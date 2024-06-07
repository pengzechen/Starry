#![allow(dead_code)]

use spinlock::SpinNoIrq;
use super::current_cpu;
use super::guest_psci::psci_ipi_handler;

extern crate alloc;
use alloc::vec::Vec;

use axhal::{interrupt_cpu_ipi_send, IPI_IRQ_NUM};

pub static IPI_HANDLER_LIST: SpinNoIrq<Vec<IpiHandler>> = SpinNoIrq::new(Vec::new());

pub static CPU_INT_LIST: SpinNoIrq<Vec<IpiMsgQueue>> = SpinNoIrq::new(Vec::new());
 
pub type IpiHandlerFunc = fn(&IpiMessage);

pub struct IpiHandler {
    pub handler: IpiHandlerFunc,
    pub ipi_type: IpiType,
}

impl IpiHandler {
    fn new(handler: IpiHandlerFunc, ipi_type: IpiType) -> IpiHandler {
        IpiHandler { handler, ipi_type }
    }
}

pub struct IpiMsgQueue {
    pub msg_queue: Vec<IpiMessage>,
}

#[derive(Debug)]
pub struct IpiMessage {
    pub ipi_type: IpiType,
    pub ipi_message: IpiInnerMsg,
}

#[derive(Copy, Clone, Debug)]
pub enum IpiType {
    Power = 0,
}

#[derive(Clone, Debug)]
pub enum IpiInnerMsg {
    // IpiTPower
    Power(IpiPowerMessage),
}

#[derive(Clone, Debug)]
pub struct IpiPowerMessage {
    pub src: usize,
    pub event: PowerEvent,
    pub entry: usize,
    pub context: usize,
}

#[allow(clippy::enum_variant_names)]
#[derive(Copy, Clone, Debug)]
pub enum PowerEvent {
    PsciIpiCpuOn,
    PsciIpiCpuOff,
}

impl IpiMsgQueue {
    pub fn default() -> IpiMsgQueue {
        IpiMsgQueue { msg_queue: Vec::new() }
    }

    pub fn push(&mut self, ipi_msg: IpiMessage) {
        self.msg_queue.push(ipi_msg);
    }

    pub fn pop(&mut self) -> Option<IpiMessage> {
        self.msg_queue.pop()
    }
}

pub fn cpu_int_list_init() {
    let mut cpu_int_list = CPU_INT_LIST.lock();
    for _ in 0..2 { // need to get cpu num by config
        cpu_int_list.push(IpiMsgQueue::default());
    }
}

pub fn ipi_register(ipi_type: IpiType, handler: IpiHandlerFunc) -> bool {
    // check handler max
    let mut ipi_handler_list = IPI_HANDLER_LIST.lock();
    for i in 0..ipi_handler_list.len() {
        if ipi_type as usize == ipi_handler_list[i].ipi_type as usize {
            debug!("ipi_register: try to cover exist ipi handler");
            return false;
        }
    }
    
    while (ipi_type as usize) >= ipi_handler_list.len() {
        ipi_handler_list.push(IpiHandler::new(handler, ipi_type));
    }
    ipi_handler_list[ipi_type as usize] = IpiHandler::new(handler, ipi_type);
    true
}

pub fn ipi_send_msg(target_id: usize, ipi_type: IpiType, ipi_message: IpiInnerMsg) -> bool {
    /* 
    let ipi_handler_list = IPI_HANDLER_LIST.lock();
    debug!("[ipi_send_msg] !!!!!!!!!!!!!!!!!!!!!!!!!!!Address of ipi_handler_list: {:p}", &*ipi_handler_list as *const _);
    debug!("[ipi_send_msg] !!!!!!!!! Address of handler: {:p}", &ipi_handler_list[0].handler as *const _);
    debug!("[ipi_send_msg] 111111111111 ipi_send_msg handler: {:#?}", ipi_handler_list[0].handler as *const());
    drop(ipi_handler_list);
    */
    // push msg to cpu int list
    let msg = IpiMessage { ipi_type, ipi_message };
    let mut cpu_int_list = CPU_INT_LIST.lock();
    cpu_int_list[target_id].msg_queue.push(msg);
    debug!("cpu_int_list {:?}", cpu_int_list[target_id].msg_queue);
    // send ipi to target core
    ipi_send(target_id)
}

fn ipi_send(target_id: usize) -> bool {
    interrupt_cpu_ipi_send(target_id, IPI_IRQ_NUM);
    true
}

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
