use spin::Mutex;
use spinlock::SpinNoIrq;

use arm_gic::gic_v2::{GicDistributor, GicHypervisorInterface, GicCpuInterface};
use arm_gic::GIC_LIST_REGS_NUM;

use crate::arch::utils::bit_extract;
use lazy_init::LazyInit;
/*
/// GICD
pub static mut GICD: LazyInit<&SpinNoIrq<GicDistributor>>= LazyInit::new();
/// GICC
pub static mut GICC: LazyInit<&GicCpuInterface>= LazyInit::new();
/// GICH
pub static mut GICH: LazyInit<&GicHypervisorInterface>= LazyInit::new();

pub const GICD_BASE: usize = 0x08000000;
pub const GICC_BASE: usize = 0x08010000;
pub const GICH_BASE: usize = 0x08030000;
pub const GICV_BASE: usize = 0x08040000;

const GIC_SGIS_NUM: usize = 16;

// GICC BITS
pub const GICC_CTLR_EN_BIT: usize = 0x1;
pub const GICC_CTLR_EOIMODENS_BIT: usize = 1 << 9;

pub static GIC_LRS_NUM: Mutex<usize> = Mutex::new(0);
*/

/// [29:28] State The state of the interrupt. This has one of the following values:
/// 00 invalid; 01 pending; 10 active; 11 pending and active.
#[derive(Copy, Clone, Debug)]
/// The state of the interrupt.
pub enum IrqState {
    /// invalid
    IrqSInactive,
    /// pending
    IrqSPend,
    /// active
    IrqSActive,
    /// pending and active
    IrqSPendActive,
}

impl IrqState {
    /// Convert a number to a state.
    pub fn num_to_state(num: usize) -> IrqState {
        match num {
            0 => IrqState::IrqSInactive,
            1 => IrqState::IrqSPend,
            2 => IrqState::IrqSActive,
            3 => IrqState::IrqSPendActive,
            _ => panic!("num_to_state: illegal irq state"),
        }
    }

    /// Convert a state to a number.
    pub fn to_num(&self) -> usize {
        match self {
            IrqState::IrqSInactive => 0,
            IrqState::IrqSPend => 1,
            IrqState::IrqSActive => 2,
            IrqState::IrqSPendActive => 3,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct GicState {
    pub saved_hcr: u32,
    saved_eisr: [u32; GIC_LIST_REGS_NUM / 32],
    saved_elrsr: [u32; GIC_LIST_REGS_NUM / 32],
    saved_apr: u32,
    pub saved_lr: [u32; GIC_LIST_REGS_NUM],
    pub saved_ctlr: u32,
}
impl GicState {
    pub fn default() -> GicState {
        GicState {
            saved_hcr: 0,
            saved_eisr: [0; GIC_LIST_REGS_NUM / 32],
            saved_elrsr: [0; GIC_LIST_REGS_NUM / 32],
            saved_apr: 0,
            saved_lr: [0; GIC_LIST_REGS_NUM],
            saved_ctlr: 0,
        }
    }

    pub fn save_state(&mut self) { 
        /*unsafe {
         
            if let Some(gich) = GICH.try_get() {
                self.saved_hcr = gich.get_hcr();
                self.saved_apr = gich.get_apr();
                // todo
                /* 
                for i in 0..(GIC_LIST_REGS_NUM / 32) {
                    self.saved_eisr[i] = gich.get_eisr_by_idx(i);
                    self.saved_elrsr[i] = gich.get_elrsr_by_idx(i);
                }*/
                for i in 0..gich.get_lrs_num() {
                    if self.saved_elrsr[0] & 1 << i == 0 {
                        self.saved_lr[i] = gich.get_lr_by_idx(i);
                    } else {
                        self.saved_lr[i] = 0;
                    }
                }
            } else {
                warn!("No available gich in save_state!")
            }
            if let Some(gicc) = GICC.try_get() {
                self.saved_ctlr = gicc.get_ctlr();
            }else {
                warn!("No available gicc in save_state!")
            }       
        }*/
    }

    pub fn restore_state(&self) {
        /*unsafe {
         
            if let Some(gich) = GICH.try_get_mut() {
                gich.set_hcr(self.saved_hcr);
                gich.set_apr(self.saved_apr);
                for i in 0..gich.get_lrs_num() {
                    gich.set_lr_by_idx(i, self.saved_lr[i]);
                }
            } else {
                warn!("No available gich in restore_state!")
            }
            if let Some(gicc) = GICC.try_get_mut() {
                gicc.set_ctlr(self.saved_ctlr);
            }else {
                warn!("No available gicc in restore_state!")
            }         
        }*/ 

    }

}
