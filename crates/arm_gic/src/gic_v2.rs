//! Types and definitions for GICv2.
//!
//! The official documentation: <https://developer.arm.com/documentation/ihi0048/latest/>

use core::ptr::NonNull;

use crate::{TriggerMode, GIC_MAX_IRQ, SPI_RANGE, SGI_RANGE, GIC_LIST_REGS_NUM, GIC_CONFIG_BITS};
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};

register_structs! {
    /// GIC Distributor registers.
    #[allow(non_snake_case)]
    GicDistributorRegs {
        /// Distributor Control Register.
        (0x0000 => CTLR: ReadWrite<u32>),
        /// Interrupt Controller Type Register.
        (0x0004 => TYPER: ReadOnly<u32>),
        /// Distributor Implementer Identification Register.
        (0x0008 => IIDR: ReadOnly<u32>),
        (0x000c => _reserved_0),
        /// Interrupt Group Registers.
        (0x0080 => IGROUPR: [ReadWrite<u32>; 0x20]),
        /// Interrupt Set-Enable Registers.
        (0x0100 => ISENABLER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Clear-Enable Registers.
        (0x0180 => ICENABLER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Set-Pending Registers.
        (0x0200 => ISPENDR: [ReadWrite<u32>; 0x20]),
        /// Interrupt Clear-Pending Registers.
        (0x0280 => ICPENDR: [ReadWrite<u32>; 0x20]),
        /// Interrupt Set-Active Registers.
        (0x0300 => ISACTIVER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Clear-Active Registers.
        (0x0380 => ICACTIVER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Priority Registers.
        (0x0400 => IPRIORITYR: [ReadWrite<u32>; 0x100]),
        /// Interrupt Processor Targets Registers.
        (0x0800 => ITARGETSR: [ReadWrite<u32>; 0x100]),
        /// Interrupt Configuration Registers.
        (0x0c00 => ICFGR: [ReadWrite<u32>; 0x40]),
        (0x0d00 => _reserved_1),
        /// Software Generated Interrupt Register.
        (0x0f00 => SGIR: WriteOnly<u32>),
        (0x0f04 => reserve2),
        (0x0f10 => CPENDSGIR: [ReadWrite<u32>; 0x4]),
        (0x0f20 => SPENDSGIR: [ReadWrite<u32>; 0x4]),
        (0x0f30 => _reserved_3),
        (0x1000 => @END),
    }
}

register_structs! {
    /// GIC CPU Interface registers.
    #[allow(non_snake_case)]
    GicCpuInterfaceRegs {
        /// CPU Interface Control Register.
        (0x0000 => CTLR: ReadWrite<u32>),
        /// Interrupt Priority Mask Register.
        (0x0004 => PMR: ReadWrite<u32>),
        /// Binary Point Register.
        (0x0008 => BPR: ReadWrite<u32>),
        /// Interrupt Acknowledge Register.
        (0x000c => IAR: ReadOnly<u32>),
        /// End of Interrupt Register.
        (0x0010 => EOIR: WriteOnly<u32>),
        /// Running Priority Register.
        (0x0014 => RPR: ReadOnly<u32>),
        /// Highest Priority Pending Interrupt Register.
        (0x0018 => HPPIR: ReadOnly<u32>),
        (0x001c => _reserved_1),
        /// CPU Interface Identification Register.
        (0x00fc => IIDR: ReadOnly<u32>),
        (0x0100 => _reserved_2),
        /// Deactivate Interrupt Register.
        (0x1000 => DIR: WriteOnly<u32>),
        (0x1004 => @END),
    }
}

// #[cfg(feature = "hv")]
/*register_structs! {
    /// GIC Hypervisor Interface registers
    #[allow(non_snake_case)]
    GicHypervisorInterfaceRegs {
        /// Hypervisor Control Register
        (0x0000 => HCR: ReadWrite<u32>),
        /// Virtual Type Register
        (0x0004 => VTR: ReadOnly<u32>),
        (0x0008 => _reserved_1),
        /// Maintenance Interrupt Status Register
        (0x0010 => MISR: ReadOnly<u32>),
        (0x0014 => reserve1),
        /// End Interrupt Status Register
        (0x0020 => EISR: [ReadOnly<u32>; GIC_LIST_REGS_NUM / 32]),
        (0x0028 => reserve2),
        /// Empty List Register Status Register
        (0x0030 => ELRSR: [ReadOnly<u32>; GIC_LIST_REGS_NUM / 32]),
        (0x0038 => reserve3),
        /// Active Priorities Registers
        (0x00f0 => APR: ReadWrite<u32>),
        (0x00f4 => reserve4),
        /// List Registers
        (0x0100 => LR: [ReadWrite<u32>; GIC_LIST_REGS_NUM]),
        (0x0200 => reserve5),
        (0x1000 => @END),
    }
}*/
register_structs! {
    #[allow(non_snake_case)]
    GicHypervisorInterfaceRegs {
        /// Hypervisor Control Register.
        (0x0000 => HCR: ReadWrite<u32>),
        /// VGIC Type Register.
        (0x0004 => VTR: ReadOnly<u32>),
        /// Virtual Machine Control Register.
        (0x0008 => VMCR: ReadWrite<u32>),
        (0x000c => _reserved_0),
        // Maintenance Interrupt Status Register.
        (0x0010 => MISR: ReadOnly<u32>),
        (0x0014 => _reserved_1),
        // End of Interrupt Status Registers 0 and 1.
        (0x0020 => EISR0: ReadOnly<u32>),
        (0x0024 => EISR1: ReadOnly<u32>),
        (0x0028 => _reserved_2),
        // Empty List Register Status Registers 0 and 1.
        (0x0030 => ELSR0: ReadOnly<u32>),
        (0x0034 => ELSR1: ReadOnly<u32>),
        (0x0038 => _reserved_3),
        // Active Priorities Register.
        (0x00f0 => APR: ReadWrite<u32>),
        (0x00f4 => _reserved_4),
        // List Registers 0-63.
        (0x0100 => LR: [ReadWrite<u32>; 0x40]),
        (0x0200 => @END),
    }
}
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
#[derive(Debug, Clone)]
pub struct GicDistributor {
    base: NonNull<GicDistributorRegs>,
    max_irqs: usize,
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
#[derive(Debug, Clone)]
pub struct GicCpuInterface {
    base: NonNull<GicCpuInterfaceRegs>,
}

/// The GIC Hypervisor interface.
///
/// Each Hypervisor interface block performs priority masking and preemption
/// handling for a connected processor in the system.
///
/// Each Hypervisor interface provides a programming interface for:
///
/// - enabling the signaling of interrupt requests to the processor
/// - acknowledging an interrupt
/// - indicating completion of the processing of an interrupt
/// - setting an interrupt priority mask for the processor
/// - defining the preemption policy for the processor
/// - determining the highest priority pending interrupt for the processor.
#[derive(Debug, Clone)]
pub struct GicHypervisorInterface {
    base: NonNull<GicHypervisorInterfaceRegs>,
}

unsafe impl Send for GicDistributor {}
unsafe impl Sync for GicDistributor {}

unsafe impl Send for GicCpuInterface {}
unsafe impl Sync for GicCpuInterface {}

unsafe impl Send for GicHypervisorInterface {}
unsafe impl Sync for GicHypervisorInterface {}

impl GicDistributor {
    /// Construct a new GIC distributor instance from the base address.
    pub const fn new(base: *mut u8) -> Self {
        Self {
            base: NonNull::new(base).unwrap().cast(),
            max_irqs: GIC_MAX_IRQ,
        }
    }

    const fn regs(&self) -> &GicDistributorRegs {
        unsafe { self.base.as_ref() }
    }

    /// The number of implemented CPU interfaces.
    pub fn cpu_num(&self) -> usize {
        ((self.regs().TYPER.get() as usize >> 5) & 0b111) + 1
    }

    /// The maximum number of interrupts that the GIC supports
    pub fn max_irqs(&self) -> usize {
        ((self.regs().TYPER.get() as usize & 0b11111) + 1) * 32
    }

    /// Configures the trigger mode for the given interrupt.
    pub fn configure_interrupt(&mut self, vector: usize, tm: TriggerMode) {
        // Only configurable for SPI interrupts
        if vector >= self.max_irqs || vector < SPI_RANGE.start {
            return;
        }

        // type is encoded with two bits, MSB of the two determine type
        // 16 irqs encoded per ICFGR register
        let reg_idx = vector >> 4;
        let bit_shift = ((vector & 0xf) << 1) + 1;
        let mut reg_val = self.regs().ICFGR[reg_idx].get();
        match tm {
            TriggerMode::Edge => reg_val |= 1 << bit_shift,
            TriggerMode::Level => reg_val &= !(1 << bit_shift),
        }
        self.regs().ICFGR[reg_idx].set(reg_val);
    }

    /// Enables or disables the given interrupt.
    pub fn set_enable(&mut self, vector: usize, enable: bool) {
        if vector >= self.max_irqs {
            return;
        }
        let reg = vector / 32;
        let mask = 1 << (vector % 32);
        if enable {
            self.regs().ISENABLER[reg].set(mask);
        } else {
            self.regs().ICENABLER[reg].set(mask);
        }
    }

    /// Set SGIR for sgi int id and target cpu.
    /* 
    pub fn set_sgi(&self, cpu_interface: usize, sgi_num: usize) {
        debug!("set sgi!!!!");
        let int_id = (sgi_num & 0b1111) as u32;
        let cpu_targetlist = 1 << (16 + cpu_interface);
        self.regs().SGIR.set(cpu_targetlist | int_id);
    }
    

    pub fn send_sgi(&mut self, cpu_if: usize, sgi_num: usize) {
        debug!("send ipi to cpu {}", cpu_if);
        self.regs().SGIR.set(((1 << (16 + cpu_if)) | (sgi_num & 0b1111)) as u32);
    }
    */
    pub fn send_sgi(&mut self, cpu_if: usize, sgi_num: usize) {
        debug!("send sgi {} with priority {:#x} to cpu {}", sgi_num, self.get_priority(sgi_num), cpu_if);
        // debug!("send sgi 2 with priority {:#x} to cpu {}", self.get_priority(2), cpu_if);
        let sgir = ((1 << (16 + cpu_if)) | (sgi_num & 0b1111)) as u32;
        debug!("this is sgir value: {:#x}", sgir);
        self.regs().SGIR.set(sgir);
    }

    /// Get interrupt priority.
    pub fn get_priority(&self, int_id: usize) -> usize {
        let idx = (int_id * 8) / 32;
        let off = (int_id * 8) % 32;
        ((self.regs().IPRIORITYR[idx].get() >> off) & 0xff) as usize
    }

    /// Set interrupt priority.
    pub fn set_priority(&mut self, int_id: usize, priority: u8) {
        let idx = (int_id * 8) / 32;
        let offset = (int_id * 8) % 32;
        let mask: u32 = 0xff << offset;

        let prev_reg_val = self.regs().IPRIORITYR[idx].get();
        // clear target int_id priority and set its priority.
        let reg_val = (prev_reg_val & !mask) | (((priority as u32) << offset) & mask);
        self.regs().IPRIORITYR[idx].set(reg_val);
    }

    /// Get interrupt target cpu.
    pub fn get_target_cpu(&self, int_id: usize) -> usize {
        let idx = (int_id * 8) / 32;
        let offset = (int_id * 8) % 32;
        ((self.regs().ITARGETSR[idx].get() >> offset) & 0xff) as usize
    }

    /// Set interrupt target cpu.
    pub fn set_target_cpu(&mut self, int_id: usize, target: u8) {
        let idx = (int_id * 8) / 32;
        let offset = (int_id * 8) % 32;
        let mask: u32 = 0xff << offset;

        let prev_reg_val = self.regs().ITARGETSR[idx].get();
        // clear target int_id target and set its target.
        let reg_val: u32 = (prev_reg_val & !mask) | (((target as u32) << offset) & mask);
        self.regs().ITARGETSR[idx].set(reg_val);
    }

    /// Set interrupt state to pending or not.
    pub fn set_pend(&self, int_id: usize, is_pend: bool, current_cpu_id: usize) {
        if SGI_RANGE.contains(&int_id) {
            let reg_idx = int_id / 4;
            let offset = (int_id % 4) * 8;
            if is_pend {
                self.regs().SPENDSGIR[reg_idx].set(1 << (offset + current_cpu_id)); // get current cpu todo()
            } else {
                self.regs().CPENDSGIR[reg_idx].set(0xff << offset);
            }
        } else {
            let reg_idx = int_id / 32;
            let mask = 1 << int_id % 32;
            if is_pend {
                self.regs().ISPENDR[reg_idx].set(mask);
            } else {
                self.regs().ICPENDR[reg_idx].set(mask);
            }
        }
    }

    /// Set interrupt state to active or not.
    pub fn set_active(&self, int_id: usize, is_active: bool) {
        let reg_idx = int_id / 32;
        let mask = 1 << int_id % 32;

        if is_active {
            self.regs().ISACTIVER[reg_idx].set(mask);
        } else {
            self.regs().ICACTIVER[reg_idx].set(mask);
        }
    }

    /// Set interrupt state. Depend on its active state and pending state.
    pub fn set_state(&self, int_id: usize, state: usize, current_cpu_id: usize) {
        self.set_active(int_id, (state & 0b10) != 0);
        self.set_pend(int_id, (state & 0b01) != 0, current_cpu_id);
    }

    /// Get interrupt state. Depend on its active state and pending state.
    pub fn get_state(&self, int_id: usize) -> usize {
        let reg_idx = int_id / 32;
        let mask = 1 << int_id % 32;

        let pend = if (self.regs().ISPENDR[reg_idx].get() & mask) != 0 {
            0b01
        } else {
            0b00
        };
        let active = if (self.regs().ISACTIVER[reg_idx].get() & mask) != 0 {
            0b10
        } else {
            0b00
        };
        return pend | active;
    }

    /// Determines whether the corresponding interrupt is edge-triggered or level-sensitive.
    pub fn set_icfgr(&self, int_id: usize, cfg: u8) {
        let reg_ind = (int_id * GIC_CONFIG_BITS) / 32;
        let off = (int_id * GIC_CONFIG_BITS) % 32;
        let mask = 0b11 << off;

        let icfgr = self.regs().ICFGR[reg_ind].get();
        self.regs().ICFGR[reg_ind].set((icfgr & !mask) | (((cfg as u32) << off) & mask));
    }


    /// Provides information about the configuration of this Redistributor.
    /// Get typer register.
    pub fn get_typer(&self) -> u32 {
        self.regs().TYPER.get()
    }

    /// Get iidr register.
    pub fn get_iidr(&self) -> u32 {
        self.regs().IIDR.get()
    }

    /// Initializes the GIC distributor.
    ///
    /// It disables all interrupts, sets the target of all SPIs to CPU 0,
    /// configures all SPIs to be edge-triggered, and finally enables the GICD.
    ///
    /// This function should be called only once.
    pub fn init(&mut self) {
        let max_irqs = self.max_irqs();
        assert!(max_irqs <= GIC_MAX_IRQ);
        self.max_irqs = max_irqs;

        // Disable all interrupts
        for i in (0..max_irqs).step_by(32) {
            self.regs().ICENABLER[i / 32].set(u32::MAX);
            self.regs().ICPENDR[i / 32].set(u32::MAX);
            self.regs().ICACTIVER[i / 32].set(u32::MAX);
        }
        if self.cpu_num() > 1 {
            for i in (SPI_RANGE.start..max_irqs).step_by(4) {
                // Set external interrupts to target cpu 0
                self.regs().IPRIORITYR[i / 4].set(u32::MAX);
                self.regs().ITARGETSR[i / 4].set(0x01_01_01_01);
            }
        }
        // Initialize all the SPIs to edge triggered
        for i in SPI_RANGE.start..max_irqs {
            self.configure_interrupt(i, TriggerMode::Edge);
        }

        // enable GIC0
        let prev = self.regs().CTLR.get();
        self.regs().CTLR.set( prev | 1 );
    }
}

impl GicCpuInterface {
    /// Construct a new GIC CPU interface instance from the base address.
    pub const fn new(base: *mut u8) -> Self {
        Self {
            base: NonNull::new(base).unwrap().cast(),
        }
    }

    const fn regs(&self) -> &GicCpuInterfaceRegs {
        unsafe { self.base.as_ref() }
    }

    // When interrupt priority drop is separated from interrupt deactivation, 
    // a write to this register deactivates the specified interrupt.
    pub fn set_dir(&self, dir: u32) {
        self.regs().DIR.set(dir);
    }
    
    /// Returns the interrupt ID of the highest priority pending interrupt for
    /// the CPU interface. (read GICC_IAR)
    ///
    /// The read returns a spurious interrupt ID of `1023` if the distributor
    /// or the CPU interface are disabled, or there is no pending interrupt on
    /// the CPU interface.
    pub fn get_iar(&self) -> u32 {
        self.regs().IAR.get()
    }

    /// Informs the CPU interface that it has completed the processing of the
    /// specified interrupt. (write GICC_EOIR)
    ///
    /// The value written must be the value returns from [`Self::iar`].
    pub fn set_eoi(&self, iar: u32) {
        self.regs().EOIR.set(iar);
    }
    
    /// Controls the CPU interface, including enabling of interrupt groups, 
    /// interrupt signal bypass, binary point registers used, and separation 
    /// of priority drop and interrupt deactivation.
    /// Get or set CTLR. 
    pub fn get_ctlr(&self) -> u32 {
        self.regs().CTLR.get()
    }
    pub fn set_ctlr(&self, ctlr: u32) {
        self.regs().CTLR.set(ctlr);
    }

    /// handles the signaled interrupt.
    ///
    /// It first reads GICC_IAR to obtain the pending interrupt ID and then
    /// calls the given handler. After the handler returns, it writes GICC_EOIR
    /// to acknowledge the interrupt.
    ///
    /// If read GICC_IAR returns a spurious interrupt ID of `1023`, it does
    /// nothing.
    pub fn handle_irq<F>(&self, handler: F)
    where
        F: FnOnce(u32),
    {
        debug!("handle_irq in crate gic!!!!!!!!!!!!!!!!");
        let iar = self.get_iar();
        let vector = iar & 0x3ff;
        if vector < 1020 {
            handler(vector);
            self.set_eoi(iar);
        } else {
            // spurious
        }
    }

    /// Initializes the GIC CPU interface.
    ///
    /// It unmask interrupts at all priority levels and enables the GICC.
    ///
    /// This function should be called only once.
    pub fn init(&self) {
        // enable GIC0
        #[cfg(not(feature = "hv"))]
        self.regs().CTLR.set(1);
        #[cfg(feature = "hv")]
        // set EOImodeNS and EN bit for hypervisor
        self.regs().CTLR.set(1);
        //self.regs().CTLR.set(1| 0x200);
        // unmask interrupts at all priority levels
        self.regs().PMR.set(0xff);
    }
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

    // ELSR1
    pub fn get_elsr1(&self) -> u32 {
        self.regs().ELSR1.get()
    }
    // ELSR0
    pub fn get_elsr0(&self) -> u32 {
        self.regs().ELSR0.get()
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

    pub fn init(&self) {
        for i in 0..self.get_lrs_num() {
            self.set_lr_by_idx(i, 0);
        }
        // LRENPIE, bit [2]: List Register Entry Not Present Interrupt Enable.
        // When it set to 1, maintenance interrupt signaled while GICH_HCR.EOICount is not 0.
        let hcr_prev = self.get_hcr();
        self.set_hcr(hcr_prev | (1 << 2) as u32);
    }
}
