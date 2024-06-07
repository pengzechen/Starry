

use arm_gicv3::regs::{
    isb, ICC_SGI1R_EL1, ICC_IAR1_EL1, ICH_HCR_EL2, ICH_VTR_EL2, ICC_SRE_EL2, ICC_SRE_EL1, ICH_VMCR_EL2, ICH_AP0R2_EL2,
    ICH_AP0R1_EL2, ICH_AP0R0_EL2, ICH_AP1R0_EL2, ICH_AP1R1_EL2, ICH_AP1R2_EL2, ICC_PMR_EL1, ICC_BPR1_EL1, ICC_CTLR_EL1,
    ICH_ELRSR_EL2, ICC_IGRPEN1_EL1, ICC_DIR_EL1, ICC_EOIR1_EL1, ICH_LR0_EL2, ICH_LR1_EL2, ICH_LR2_EL2, ICH_LR3_EL2,
    ICH_LR4_EL2, ICH_LR5_EL2, ICH_LR6_EL2, ICH_LR7_EL2, ICH_LR8_EL2, ICH_LR9_EL2, ICH_LR10_EL2, ICH_LR11_EL2,
    ICH_LR12_EL2, ICH_LR13_EL2, ICH_LR14_EL2, ICH_LR15_EL2, ICH_EISR_EL2, ICH_MISR_EL2,
};

use arm_gicv3::{
    GIC_PRIVATE_INT_NUM, GIC_LIST_REGS_NUM, GICH_VTR_PRIBITS_OFF, GICH_VTR_PRIBITS_LEN, 
    GICC_CTLR_EOIMODE_BIT, GICC_IGRPEN_EL1_ENB_BIT, GICC_SRE_EL2_ENABLE,
    GICH, GICR
};

use arm_gicv3::regs::ReadableReg;
use arm_gicv3::regs::WriteableReg;

use arm_gicv3::{gich_lrs_num, gic_set_act, gic_set_pend};

#[derive(Copy, Clone, Debug)] pub enum IrqState {
    IrqSInactive,
    IrqSPend,
    IrqSActive,
    IrqSPendActive,
}

impl IrqState {
    pub fn num_to_state(num: usize) -> IrqState {
        match num {
            0 => IrqState::IrqSInactive,
            1 => IrqState::IrqSPend,
            2 => IrqState::IrqSActive,
            3 => IrqState::IrqSPendActive,
            _ => panic!("num_to_state: illegal irq state"),
        }
    }

    pub fn to_num(&self) -> usize {
        match self {
            IrqState::IrqSInactive => 0,
            IrqState::IrqSPend => 1,
            IrqState::IrqSActive => 2,
            IrqState::IrqSPendActive => 3,
        }
    }
}

pub fn gic_set_state(int_id: usize, state: usize, gicr_id: u32) {
    gic_set_act(int_id, (state & IrqState::IrqSActive as usize) != 0, gicr_id);
    gic_set_pend(int_id, (state & IrqState::IrqSPend as usize) != 0, gicr_id);
}



/// GIC state struct
#[repr(C)] #[derive(Debug, Copy, Clone)] pub struct GicState {
    pub ctlr: u32,
    pub pmr: u32,
    pub bpr: u32,
    pub eoir: u32,
    pub rpr: u32,
    pub hppir: u32,
    pub priv_isenabler: u32,
    pub priv_ipriorityr: [u32; GIC_PRIVATE_INT_NUM / 4],
    pub hcr: usize,
    pub lr: [usize; GIC_LIST_REGS_NUM],
    pub apr0: [u32; 4],
    pub apr1: [u32; 4],
    igrpen1: usize,
    vmcr: u32,
    nr_prio: u32, //Priority bits. The number of virtual priority bits implemented, minus one.
    sre_el1: u32,
}

/*  need to set priv_isenabler */
impl GicState {
    pub fn default() -> Self {
        let nr_prio = (((ICH_VTR_EL2::read() >> GICH_VTR_PRIBITS_OFF) & ((1 << GICH_VTR_PRIBITS_LEN) - 1)) + 1) as u32;
        GicState {
            ctlr: GICC_CTLR_EOIMODE_BIT as u32,
            igrpen1: GICC_IGRPEN_EL1_ENB_BIT,
            pmr: 0xff,
            bpr: 0,
            eoir: 0,
            rpr: 0,
            hppir: 0,
            // priv_isenabler: GICR[current_cpu().id].ISENABLER0.get(), pzc change 
            priv_isenabler: 0,
            priv_ipriorityr: [u32::MAX; GIC_PRIVATE_INT_NUM / 4],
            hcr: 0b101,
            lr: [0; GIC_LIST_REGS_NUM],
            vmcr: 0,
            nr_prio,
            apr0: [0; 4],
            apr1: [0; 4],
            sre_el1: 0,
        }
    }

    pub fn save_state(&mut self) {}

    pub fn restore_state(&self) {}
}

trait InterruptContextTrait {
    fn save_state(&mut self);
    fn restore_state(&self);
}

impl InterruptContextTrait for GicState {
    fn save_state(&mut self) {
        self.hcr = ICH_HCR_EL2::read();
        // save VMCR_EL2: save and restore the virtual machine view of the GIC state.
        self.vmcr = ICH_VMCR_EL2::read() as u32;
        // save ICH_AP1Rn_EL2: Provides information about Group 1 virtual active priorities for EL2.
        // if some bit set 1:There is a Group 1 interrupt active with this priority level which has not undergone priority drop.
        self.save_aprn_regs();
        // save lr
        for i in 0..gich_lrs_num() {
            self.lr[i] = GICH.lr(i);
        }
        // save ICC_SRE_EL1: EL1`s systregister use
        self.sre_el1 = ICC_SRE_EL1::read() as u32;
        // SAFETY: change the value of ICC_SRE_EL2 without GICC_SRE_EL2_ENABLE_bit
        unsafe { ICC_SRE_EL2::write(ICC_SRE_EL2::read() & !GICC_SRE_EL2_ENABLE) }
    }

    fn restore_state(&self) {
        // make EL2 can use sysrem register
        // SAFETY:
        // Set Enable[3] bit to 1, and set the SRE[0] bits to 1
        // And other bits set to 0
        unsafe {
            ICC_SRE_EL2::write(0b1001);
        }
        // restore ICC_SRE_EL1 for EL1
        // SAFETY:
        // Set the SRE[0] bits to 1
        // And other bits set to 0
        unsafe {
            ICC_SRE_EL1::write(0x1);
        }
        isb();
        // SAFETY: The value is saved last time
        unsafe {
            // restore HCR
            ICH_HCR_EL2::write(self.hcr);
            // restore ICH_VMCR_EL2
            ICH_VMCR_EL2::write(self.vmcr as usize);
        }
        // restore aprn
        self.restore_aprn_regs();
        // restore lr
        for i in 0..gich_lrs_num() {
            GICH.set_lr(i, self.lr[i]);
        }
    }
}

impl GicState {
    fn save_apr2(&mut self) {
        self.apr0[2] = ICH_AP0R2_EL2::read() as u32;
        self.apr1[2] = ICH_AP1R2_EL2::read() as u32;
    }

    fn save_apr1(&mut self) {
        self.apr0[1] = ICH_AP0R1_EL2::read() as u32;
        self.apr1[1] = ICH_AP1R1_EL2::read() as u32;
    }

    fn save_apr0(&mut self) {
        self.apr0[0] = ICH_AP0R0_EL2::read() as u32;
        self.apr1[0] = ICH_AP1R0_EL2::read() as u32;
    }

    fn save_aprn_regs(&mut self) {
        match self.nr_prio {
            7 => {
                self.save_apr2();
                self.save_apr1();
                self.save_apr0();
            }
            6 => {
                self.save_apr1();
                self.save_apr0();
            }
            5 => {
                self.save_apr0();
            }
            _ => panic!("priority not surpport"),
        }
    }

    fn restore_aprn_regs(&self) {
        // SAFETY: All value is saved last time
        let restore_apr2 = || unsafe {
            ICH_AP0R2_EL2::write(self.apr0[2] as usize);
            ICH_AP1R2_EL2::write(self.apr1[2] as usize);
        };
        let restore_apr1 = || unsafe {
            ICH_AP0R1_EL2::write(self.apr0[1] as usize);
            ICH_AP1R1_EL2::write(self.apr1[1] as usize);
        };
        let restore_apr0 = || unsafe {
            ICH_AP0R0_EL2::write(self.apr0[0] as usize);
            ICH_AP1R0_EL2::write(self.apr1[0] as usize);
        };
        match self.nr_prio {
            7 => {
                restore_apr2();
                restore_apr1();
                restore_apr0();
            }
            6 => {
                restore_apr1();
                restore_apr0();
            }
            5 => {
                restore_apr0();
            }
            _ => panic!("priority not surpport"),
        }
    }
}

