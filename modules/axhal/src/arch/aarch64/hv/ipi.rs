use spin::Mutex;
extern crate alloc;
use alloc::vec::Vec;

use crate::platform::aarch64_common::gic::*;

pub static IPI_HANDLER_LIST: Mutex<Vec<IpiHandler>> = Mutex::new(Vec::new());

pub static CPU_INT_LIST: Mutex<Vec<IpiMsgQueue>> = Mutex::new(Vec::new());
 
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
    // interrupt_cpu_ipi_send(0, 15);
    // interrupt_cpu_ipi_send(1, 15);
    true
}
/* 
pub fn ipi_send_msg(target_id: usize, ipi_type: IpiType, ipi_message: IpiInnerMsg) -> bool {
    let msg = IpiMessage { ipi_type, ipi_message };
    ipi_send(target_id, msg)
}

fn ipi_send(target_id: usize, msg: IpiMessage) -> bool {
    // CPU_INT_LIST[target_id].lock().push(msg);
    interrupt_cpu_ipi_send(target_id, IPI_IRQ_NUM);

    true
}

fn ipi_pop_message(cpu_id: usize) -> Option<IpiMessage> {
    // let mut cpu_if = CPU_INT_LIST[cpu_id].lock();
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
}*/
