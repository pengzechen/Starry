//! Types and definitions for GICv2.
//!
//! The official documentation: <https://developer.arm.com/documentation/ihi0048/latest/>

use core::ptr::NonNull;

use crate::registers::gicv2_regs::*;

use crate::{GenericArmGic, IntId, TriggerMode};
use tock_registers::interfaces::{Readable, Writeable};

/// The GIC distributor.
///
/// The Distributor block performs interrupt prioritization and distribution
/// to the CPU interface blocks that connect to the processors in the system.
///
/// The Distributor provides a programming interface for:
/// - Globally enabling the forwarding of interrupts to the CPU interfaces.
/// - Enabling or disabling each interrupt.
/// - Setting the priority level of each interrupt.
/// - Setting the target processor list of each interrupt.
/// - Setting each peripheral interrupt to be level-sensitive or edge-triggered.
/// - Setting each interrupt as either Group 0 or Group 1.
/// - Forwarding an SGI to one or more target processors.
///
/// In addition, the Distributor provides:
/// - visibility of the state of each interrupt
/// - a mechanism for software to set or clear the pending state of a peripheral
///   interrupt.
#[derive(Debug, Copy, Clone)]
pub struct GicDistributor {
    base: NonNull<GicDistributorRegs>,
    support_irqs: usize,
    #[allow(dead_code)]
    support_cpu: usize,
}

impl GicDistributor {
    const GICD_DISABLE: u32 = 0;
    const GICD_ENABLE: u32 = 1;

    const CPU_NUM_SHIFT: usize = 5;
    const CPU_NUM_MASK: u32 = 0b111;
    const IT_LINES_NUM_MASK: u32 = 0b11111;

    /// Construct a new GIC distributor instance from the base address.
    pub const fn new(base: *mut u8) -> Self {
        Self {
            base: NonNull::new(base).unwrap().cast(),
            support_irqs: 0,
            support_cpu: 0,
        }
    }

    const fn regs(&self) -> &GicDistributorRegs {
        unsafe { self.base.as_ref() }
    }

    /// Configures the trigger type for the interrupt with the given ID.
    fn set_trigger(&mut self, id: usize, tm: TriggerMode) {
        // type is encoded with two bits, MSB of the two determine type
        // 16 irqs encoded per ICFGR register
        let index = id >> 4;
        let bit_shift = ((id & 0xf) << 1) + 1;

        let mut reg_val = self.regs().ICFGR[index].get();
        match tm {
            TriggerMode::Edge => reg_val |= 1 << bit_shift,
            TriggerMode::Level => reg_val &= !(1 << bit_shift),
        }

        self.regs().ICFGR[index].set(reg_val);
    }

    /// Initializes the GIC distributor.
    ///
    /// It disables all interrupts, sets the target of all SPIs to CPU 0,
    /// configures all SPIs to be edge-triggered, and finally enables the GICD.
    ///
    /// This function should be called only once.
    pub fn init(&mut self) {
        let typer = self.regs().TYPER.get();

        // The maximum number of interrupts that the GIC supports
        // If ITLinesNumber=N, the maximum number of interrupts is 32(N+1)
        let irq_num = (((typer & Self::IT_LINES_NUM_MASK) + 1) * 32) as usize;
        match irq_num {
            0..=IntId::GIC_MAX_IRQ => self.support_irqs = irq_num,
            _ => self.support_irqs = IntId::GIC_MAX_IRQ,
        }

        self.support_cpu = (((typer >> Self::CPU_NUM_SHIFT) & Self::CPU_NUM_MASK) + 1) as usize;

        // disable GICD
        self.regs().CTLR.set(Self::GICD_DISABLE);

        // Set all global interrupts to CPU0.
        for i in (IntId::SPI_START..self.support_irqs).step_by(4) {
            // Set external interrupts to target cpu 0
            // once time set 4 interrupts
            self.regs().ITARGETSR[i / 4].set(0x01_01_01_01);
        }

        // Initialize all the SPIs to edge triggered
        for i in IntId::SPI_START..self.support_irqs {
            self.set_trigger(i, TriggerMode::Edge);
        }

        // Set priority on all global interrupts
        for i in (IntId::SPI_START..self.support_irqs).step_by(4) {
            // once time set 4 interrupts
            self.regs().IPRIORITYR[i / 4].set(0xa0_a0_a0_a0);
        }

        // Deactivate and disable all SPIs
        for i in (IntId::SPI_START..self.support_irqs).step_by(32) {
            self.regs().ICACTIVER[i / 32].set(u32::MAX);
            self.regs().ICENABLER[i / 32].set(u32::MAX);
        }

        // enable GIC0
        self.regs().CTLR.set(Self::GICD_ENABLE);
    }
}

/// The GIC CPU interface.
///
/// Each CPU interface block performs priority masking and preemption
/// handling for a connected processor in the system.
///
/// Each CPU interface provides a programming interface for:
///
/// - enabling the signaling of interrupt requests to the processor
/// - acknowledging an interrupt
/// - indicating completion of the processing of an interrupt
/// - setting an interrupt priority mask for the processor
/// - defining the preemption policy for the processor
/// - determining the highest priority pending interrupt for the processor.
#[derive(Debug, Copy, Clone)]
pub struct GicCpuInterface {
    base: NonNull<GicCpuInterfaceRegs>,
}

impl GicCpuInterface {
    const GICC_ENABLE: u32 = 1;

    /// Construct a new GIC CPU interface instance from the base address.
    pub const fn new(base: *mut u8) -> Self {
        Self {
            base: NonNull::new(base).unwrap().cast(),
        }
    }

    const fn regs(&self) -> &GicCpuInterfaceRegs {
        unsafe { self.base.as_ref() }
    }

    /// Initializes the GIC CPU interface.
    ///
    /// It unmask interrupts at all priority levels and enables the GICC.
    ///
    /// This function should be called only once.
    pub fn init(&self, gicd: &GicDistributor) {
        // Deactivate and disable all private interrupts
        gicd.regs().ICACTIVER[0].set(u32::MAX);
        gicd.regs().ICENABLER[0].set(u32::MAX);

        // Set priority on private interrupts
        for i in (0..IntId::SPI_START).step_by(4) {
            // once time set 4 interrupts
            gicd.regs().IPRIORITYR[i / 4].set(0xa0_a0_a0_a0);
        }

        // unmask interrupts at all priority levels
        self.regs().PMR.set(0xff);
        // enable GIC0
        self.regs().CTLR.set(Self::GICC_ENABLE);
    }
}

unsafe impl Send for GicDistributor {}
unsafe impl Sync for GicDistributor {}

unsafe impl Send for GicCpuInterface {}
unsafe impl Sync for GicCpuInterface {}

/// Driver for an Arm Generic Interrupt Controller version 2.
#[derive(Debug, Copy, Clone)]
pub struct GicV2 {
    gicd: GicDistributor,
    gicc: GicCpuInterface,
}

unsafe impl Send for GicV2 {}
unsafe impl Sync for GicV2 {}

impl GicV2 {
    /// # Safety
    ///
    /// The given base addresses must point to the GIC distributor and redistributor registers
    /// respectively. These regions must be mapped into the address space of the process as device
    /// memory, and not have any other aliases, either via another instance of this driver or
    /// otherwise.
    pub const fn new(gicd: *mut u8, gicc: *mut u8) -> Self {
        Self {
            gicd: GicDistributor::new(gicd),
            gicc: GicCpuInterface::new(gicc),
        }
    }
}

impl GenericArmGic for GicV2 {
    /// Initialises the GIC.
    fn init_primary(&mut self) {
        self.gicd.init();
        self.gicc.init(&self.gicd);
    }

    /// Initialises the GIC for the current CPU core.
    fn per_cpu_init(&mut self) {
        self.gicc.init(&self.gicd);
    }

    /// Configures the trigger type for the interrupt with the given ID.
    fn set_trigger(&mut self, intid: IntId, tm: TriggerMode) {
        // Only configurable for SPI interrupts
        if intid.0 < IntId::SPI_START {
            return;
        }
        self.gicd.set_trigger(intid.0, tm);
    }

    /// Enables the interrupt with the given ID.
    fn enable_interrupt(&mut self, intid: IntId) {
        let index = intid.0 / 32;
        let bit = 1 << (intid.0 % 32);
        self.gicd.regs().ISENABLER[index].set(bit);
    }

    /// Disable the interrupt with the given ID.
    fn disable_interrupt(&mut self, intid: IntId) {
        let index = intid.0 / 32;
        let bit = 1 << (intid.0 % 32);
        self.gicd.regs().ICENABLER[index].set(bit);
    }

    fn get_and_acknowledge_interrupt(&self) -> Option<IntId> {
        let iar = self.gicc.regs().IAR.get();
        let id = (iar & 0x3ff) as usize;
        if id >= IntId::SPECIAL_START {
            None
        } else {
            Some(IntId(id))
        }
    }

    /// Informs the interrupt controller that the CPU has completed processing the given interrupt.
    /// This drops the interrupt priority and deactivates the interrupt.
    fn end_interrupt(&self, intid: IntId) {
        self.gicc.regs().EOIR.set(intid.0 as u32);
    }
}

// pzc add 5.1

use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite};

pub const GIC_LIST_REGS_NUM: usize = 64;

register_structs! {
    /// GIC Hypervisor Interface registers
    #[allow(non_snake_case)]
    GicHypervisorInterfaceRegs {
        /// Hypervisor Control Register
        (0x0000 => HCR: ReadWrite<u32>),
        /// Virtual Type Register
        (0x0004 => VTR: ReadOnly<u32>),
        /// Virtual Machine Control Register
        (0x0008 => VMCR: ReadWrite<u32>),
        (0x000c => _reserved_0),
        /// Maintenance Interrupt Status Register
        (0x0010 => MISR: ReadOnly<u32>),
        (0x0014 => _reserved_1),
        /// End Interrupt Status Register
        (0x0020 => EISR: [ReadOnly<u32>; GIC_LIST_REGS_NUM / 32]),
        (0x0028 => _reserved_2),
        /// Empty List Register Status Register
        (0x0030 => ELRSR: [ReadOnly<u32>; GIC_LIST_REGS_NUM / 32]),
        (0x0038 => _reserved_3),
        /// Active Priorities Registers
        (0x00f0 => APR: ReadWrite<u32>),
        (0x00f4 => _reserved_4),
        /// List Registers
        (0x0100 => LR: [ReadWrite<u32>; GIC_LIST_REGS_NUM]),
        (0x0200 => _reserved_5),
        (0x1000 => @END),
    }
}

#[derive(Debug, Clone)]
pub struct GicHypervisorInterface {
    base: NonNull<GicHypervisorInterfaceRegs>,
}

impl GicHypervisorInterface {
    /// Construct a new GIC hypervisor interface instance from the base address.
    pub const fn new(base: *mut u8) -> Self {
        Self {
            base: NonNull::new(base).unwrap().cast(),
        }
    }

    const fn regs(&self) -> &GicHypervisorInterfaceRegs {
        unsafe { self.base.as_ref() }
    }

    // HCR: Controls the virtual CPU interface.
    // Get or set HCR.
    pub fn get_hcr(&self) -> u32 {
        self.regs().HCR.get()
    }
    pub fn set_hcr(&self, hcr: u32) {
        self.regs().HCR.set(hcr);
    }

    // Enables the hypervisor to save and restore the virtual machine view of the GIC state.
    pub fn get_vmcr(&self) -> u32 {
        self.regs().VMCR.get()
    }
    pub fn set_vmcr(&self, vmcr:u32) {
        self.regs().VMCR.set(vmcr);
    }
    // VTR: Indicates the number of implemented virtual priority bits and List registers.
    // VTR ListRegs, bits [4:0]: The number of implemented List registers, minus one.
    // Get ListRegs number.
    #[inline(always)]
    pub fn get_lrs_num(&self) -> usize {
        let vtr = self.regs().VTR.get();
        ((vtr & 0b11111) + 1) as usize
    }

    // LR<n>: These registers provide context information for the virtual CPU interface.
    // Get or set LR by index.
    pub fn get_lr_by_idx(&self, lr_idx: usize) -> u32 {
        self.regs().LR[lr_idx].get()
    }
    pub fn set_lr_by_idx(&self, lr_idx: usize, val: u32) {
        self.regs().LR[lr_idx].set(val)
    }

    // MISR: Indicates which maintenance interrupts are asserted.
    // Get MISR.
    pub fn get_misr(&self) -> u32 {
        self.regs().MISR.get()
    }

    // APR: These registers track which preemption levels are active in the virtual CPU interface,
    //      and indicate the current active priority. Corresponding bits are set to 1 in this register
    //      when an interrupt is acknowledged, based on GICH_LR<n>.Priority, and the least significant
    //      bit set is cleared on EOI.
    // Get or set APR.
    pub fn get_apr(&self) -> u32 {
        self.regs().APR.get()
    }
    pub fn set_apr(&self, apr: u32) {
        self.regs().APR.set(apr);
    }

    pub fn get_eisr_by_idx(&self, eisr_idx: usize) -> u32 {
        self.regs().EISR[eisr_idx].get()
    }

    pub fn get_elrsr_by_idx(&self, elsr_idx: usize) -> u32 {
        self.regs().ELRSR[elsr_idx].get()
    }

    pub fn init(&self) {
        for i in 0..self.get_lrs_num() {
            self.set_lr_by_idx(i, 0);
        }
        // [9] VEM Alias of GICV_CTLR.EOImode.
        self.set_vmcr(1 | 1 << 9);
        // LRENPIE, bit [2]: List Register Entry Not Present Interrupt Enable. When it set to 1, maintenance interrupt signaled while GICH_HCR.EOICount is not 0.
        let hcr_prev: u32 = self.get_hcr();
        self.set_hcr(hcr_prev | 1 as u32 | (1 << 2) as u32);    // need to set bit 0????? [0] enable maintenance interrupt
    }
}
