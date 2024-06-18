
use spin::RwLock;
extern crate alloc;
use alloc::vec::Vec;
use super::{active_vm, current_cpu};
use hypercraft::arch::emu::{EmuContext, EmuDevs};
use hypercraft::arch::emu::*;

pub fn vgic_icc_sre_handler(_emu_dev_id: usize, emu_ctx: &EmuContext) -> bool {
    if !emu_ctx.write {
        current_cpu().set_gpr(emu_ctx.reg, 0x1);
    }
    true
}

/// Static RwLock containing a vector of EmuRegEntry instances, representing emulator registers.
static EMU_REGS_LIST: RwLock<Vec<EmuRegEntry>> = RwLock::new(Vec::new());

/// Handles emulator register operations based on the provided EmuContext.
pub fn emu_reg_handler(emu_ctx: &EmuContext) -> bool {
    let address = emu_ctx.address;
    let active_vcpu = current_cpu().active_vcpu.as_ref().unwrap();
    let vm_id = active_vcpu.vm_id;

    let emu_regs_list = EMU_REGS_LIST.read();
    for emu_reg in emu_regs_list.iter() {
        if emu_reg.addr == address {
            let handler = emu_reg.handler;
            drop(emu_regs_list);
            return handler(vm_id, emu_ctx);
        }
    }
    error!(
        "emu_reg_handler: no handler for Core{} {} reg ({:#x})",
        current_cpu().cpu_id,
        if emu_ctx.write { "write" } else { "read" },
        address
    );
    false
}

/// Registers a new emulator register with the specified type, address, and handler function.
pub fn emu_register_reg(emu_type: EmuRegType, address: usize, handler: EmuRegHandler) {
    let mut emu_regs_list = EMU_REGS_LIST.write();

    for emu_reg in emu_regs_list.iter() {
        if address == emu_reg.addr {
            warn!(
                "emu_register_reg: duplicated emul reg addr: prev address {:#x}",
                address
            );
            return;
        }
    }

    emu_regs_list.push(EmuRegEntry {
        emu_type,
        addr: address,
        handler,
    });
}

/// Type alias for the handler function of emulator registers.
type EmuRegHandler = EmuDevHandler;

/// Struct representing an entry in the emulator register list.
pub struct EmuRegEntry {
    /// The type of the emulator register.
    pub emu_type: EmuRegType,
    /// The address associated with the emulator register.
    pub addr: usize,
    /// The handler function for the emulator register.
    pub handler: EmuRegHandler,
}

/// Enumeration representing the type of emulator registers.
pub enum EmuRegType {    /// System register type for emulator registers.
    SysReg,
}
