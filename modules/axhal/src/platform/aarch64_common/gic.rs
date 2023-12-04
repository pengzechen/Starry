use crate::{irq::IrqHandler, mem::phys_to_virt};
use arm_gic::gic_v2::{GicCpuInterface, GicDistributor, GicHypervisorInterface};
use memory_addr::PhysAddr;
use spinlock::SpinNoIrq;

#[cfg(feature = "hv")]
use hypercraft;

#[cfg(feature = "hv")]
use hypercraft::arch::utils::bit_extract;

/// The maximum number of IRQs.
pub const MAX_IRQ_COUNT: usize = 1024;

/// The timer IRQ number.
pub const TIMER_IRQ_NUM: usize = 30; // physical timer, type=PPI, id=14

/// The hypervisor timer irq number.
pub const HYPERVISOR_TIMER_IRQ_NUM: usize = 26;

/// The ipi irq number.
pub const IPI_IRQ_NUM: usize = 1;

/// The maintenance interrupt irq number.
pub const MAINTENANCE_IRQ_NUM: usize = 25;

pub const GIC_SGIS_NUM: usize = 16;

const GICD_BASE: PhysAddr = PhysAddr::from(axconfig::GICD_PADDR);
const GICC_BASE: PhysAddr = PhysAddr::from(axconfig::GICC_PADDR);
#[cfg(feature = "hv")]
const GICH_BASE: PhysAddr = PhysAddr::from(axconfig::GICH_PADDR);

#[cfg(feature = "hv")]
const LR_VIRTIRQ_MASK: usize = 0x3ff;
#[cfg(feature = "hv")]
const LR_PENDING_BIT: u32 = 1 << 28;
#[cfg(feature = "hv")]
const LR_PHYSIRQ_MASK: usize = 0x3ff << 10;
#[cfg(feature = "hv")]
const LR_HW_BIT: u32 = 1 << 31;

pub static GICD: SpinNoIrq<GicDistributor> =
    SpinNoIrq::new(GicDistributor::new(phys_to_virt(GICD_BASE).as_mut_ptr()));

// per-CPU, no lock
pub static GICC: GicCpuInterface = GicCpuInterface::new(phys_to_virt(GICC_BASE).as_mut_ptr());

#[cfg(feature = "hv")]
pub static GICH: GicHypervisorInterface = GicHypervisorInterface::new(phys_to_virt(GICH_BASE).as_mut_ptr());

/// Enables or disables the given IRQ.
pub fn set_enable(irq_num: usize, enabled: bool) {
    #[cfg(not(feature = "hv"))]
    GICD.lock().set_enable(irq_num as _, enabled);
    #[cfg(feature = "hv")]
    {
        debug!("in platform gic set_enable: irq_num {}, enabled {}", irq_num, enabled);
        GICD.lock().set_priority(irq_num as _, 0x7f);
        GICD.lock().set_target_cpu(irq_num as _, 1 << 0);   // only enable one cpu
        GICD.lock().set_enable(irq_num as _, enabled);
    }
}

/// Registers an IRQ handler for the given IRQ.
///
/// It also enables the IRQ if the registration succeeds. It returns `false` if
/// the registration failed.
pub fn register_handler(irq_num: usize, handler: IrqHandler) -> bool {
    crate::irq::register_handler_common(irq_num, handler)
}

#[cfg(not(feature = "hv"))]
/// Dispatches the IRQ.
///
/// This function is called by the common interrupt handler. It looks
/// up in the IRQ handler table and calls the corresponding handler. If
/// necessary, it also acknowledges the interrupt controller after handling.
pub fn dispatch_irq(_unused: usize) {
    GICC.handle_irq(|irq_num| crate::irq::dispatch_irq_common(irq_num as _));
}

#[cfg(feature = "hv")]
pub fn dispatch_irq(irq_num: usize) {
    debug!("dispatch_irq_hv: irq_num {}", irq_num);
    crate::irq::dispatch_irq_common(irq_num as _);
}

/// Initializes GICD, GICC on the primary CPU.
pub(crate) fn init_primary() {
    info!("Initialize GICv2...");
    GICD.lock().init();
    GICC.init();
    #[cfg(feature = "hv")]
    {
        // GICH.init();
    }
}

/// Initializes GICC on secondary CPUs.
#[cfg(feature = "smp")]
pub(crate) fn init_secondary() {
    info!("Initialize init_secondary GICv2...");
    GICC.init();
}

#[cfg(feature = "hv")]
pub fn gicc_get_current_irq() -> (usize, usize) {
    let iar = GICC.get_iar();
    let irq = iar as usize;
    // current_cpu().current_irq = irq;
    let irq = bit_extract(irq, 0, 10);
    let src = bit_extract(irq, 10, 3);
    (irq, src)
}

#[cfg(feature = "hv")]
#[no_mangle]
pub fn interrupt_cpu_ipi_send(cpu_id: usize, ipi_id: usize) {
    debug!("interrupt_cpu_ipi_send: cpu_id {}, ipi_id {}", cpu_id, ipi_id);
    if ipi_id < GIC_SGIS_NUM {
        GICD.lock().set_sgi(cpu_id, ipi_id);
    }
}

#[cfg(feature = "hv")]
pub fn pending_irq() -> Option<usize> {
    let iar = GICC.get_iar();
    debug!("this is iar:{:#x}", iar);
    if iar >= 0x3fe {
        // spurious
        None
    } else {
        Some(iar as _)
    }
}

#[cfg(feature = "hv")]
pub fn deactivate_irq(iar: usize) {
    GICC.set_eoi(iar as _);    
}

#[cfg(feature = "hv")]
pub fn inject_irq(irq_id: usize) {
    let elsr: u64 = (GICH.get_elsr1() as u64) << 32 | GICH.get_elsr0() as u64;
    let lr_num = GICH.get_lrs_num();
    let mut lr_idx = -1 as isize;
    for i in 0..lr_num {
        if (1 << i) & elsr > 0 {
            if lr_idx == -1 {
                lr_idx = i as isize;
            }
            continue;
        }
        // overlap
        let _lr_val = GICH.get_lr_by_idx(i) as usize;
        if (i & LR_VIRTIRQ_MASK) == irq_id {
            return;
        }
    }
    debug!("To Inject IRQ {:#x}, find lr {}", irq_id, lr_idx);
    if lr_idx == -1 {
        return;
    } else {
        let mut val = 0;
    
        val = irq_id as u32;
        val |= LR_PENDING_BIT;
    
        if false
        /* sgi */
        {
            todo!()
        } else {
            val |= ((irq_id << 10) & LR_PHYSIRQ_MASK) as u32;
            val |= LR_HW_BIT;
        }   
                
        debug!("To write lr {:#x} val {:#x}", lr_idx, val);
        GICH.set_lr_by_idx(lr_idx as usize, val);
    }
}
