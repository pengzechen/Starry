use crate::{irq::IrqHandler, mem::phys_to_virt};
use arm_gic::gic_v2::{
    GicCpuInterface, GicDistributor, GicHypervisorInterface, GicVcpuInterface
};
use arm_gic::{GIC_SGIS_NUM, GIC_PRIVATE_INT_NUM};
use memory_addr::PhysAddr;
use spinlock::SpinNoIrq;
use spin::Mutex;

/// The maximum number of IRQs.
pub const MAX_IRQ_COUNT: usize = 1024;
#[cfg(feature = "hv")]
pub const GIC_SPI_MAX: usize = MAX_IRQ_COUNT - GIC_PRIVATE_INT_NUM;

#[cfg(feature = "hv")]
use hypercraft::arch::utils::bit_extract;

/// The timer IRQ number.
pub const TIMER_IRQ_NUM: usize = 30; // physical timer, type=PPI, id=14

#[cfg(feature = "hv")]
/// The hypervisor timer irq number.
pub const HYPERVISOR_TIMER_IRQ_NUM: usize = 26;

#[cfg(feature = "hv")]
/// The ipi irq number.
pub const IPI_IRQ_NUM: usize = 1;

#[cfg(feature = "hv")]
/// The maintenance interrupt irq number.
pub const MAINTENANCE_IRQ_NUM: usize = 25;

pub const GICD_BASE: PhysAddr = PhysAddr::from(axconfig::GICD_PADDR);
const GICC_BASE: PhysAddr = PhysAddr::from(axconfig::GICC_PADDR);
#[cfg(feature = "hv")]
const GICH_BASE: PhysAddr = PhysAddr::from(axconfig::GICH_PADDR);
#[cfg(feature = "hv")]
const GICV_BASE: PhysAddr = PhysAddr::from(0x8040000);

pub static GICD: SpinNoIrq<GicDistributor> =
    SpinNoIrq::new(GicDistributor::new(phys_to_virt(GICD_BASE).as_mut_ptr()));

// per-CPU, no lock
pub static GICC: GicCpuInterface = GicCpuInterface::new(phys_to_virt(GICC_BASE).as_mut_ptr());

#[cfg(feature = "hv")]
pub static GICH: GicHypervisorInterface = GicHypervisorInterface::new(phys_to_virt(GICH_BASE).as_mut_ptr());

#[cfg(feature = "hv")]
pub static GICV: GicVcpuInterface = GicVcpuInterface::new(phys_to_virt(GICV_BASE).as_mut_ptr());

#[cfg(feature = "hv")]
pub static GIC_LRS_NUM: Mutex<usize> = Mutex::new(0);

/// Enables or disables the given IRQ.
pub fn set_enable(irq_num: usize, enabled: bool) {
    debug!("in platform gic set_enable: irq_num {}, enabled {}", irq_num, enabled);
    GICD.lock().set_enable(irq_num as _, enabled);

    #[cfg(feature = "hv")]
    GICD.lock().set_priority(irq_num as _, 0x7f);
    /* 
    #[cfg(feature = "hv")]
    {
        debug!("in platform gic set_enable: irq_num {}, enabled {}", irq_num, enabled);
        // GICD.lock().set_priority(irq_num as _, 0x7f);
        // GICD.lock().set_target_cpu(irq_num as _, 1 << 0);   // only enable one cpu
        GICD.lock().set_enable(irq_num as _, enabled);
    }
    */
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
    // GICD.lock().init();
    // GICC.init();
    gic_global_init();
    gic_local_init();
}

/// Initializes GICC on secondary CPUs.
#[cfg(feature = "smp")]
pub(crate) fn init_secondary() {
    info!("Initialize init_secondary GICv2...");
    // GICC.init();
    gic_local_init();
}

fn gic_global_init() {
    set_gic_lrs(GICH.get_lrs_num());
    GICD.lock().global_init();
}

fn gic_local_init() {
    GICD.lock().local_init();
    GICC.init();
    #[cfg(feature = "hv")]
    GICH.init();

    let ctlr = GICC.get_ctlr();
}

#[cfg(feature = "hv")]
pub fn gicc_get_current_irq() -> (usize, usize) {
    let iar = GICC.get_iar();
    let irq = iar as usize;
    // debug!("this is iar:{:#x}", iar);
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
       GICD.lock().send_sgi(cpu_id, ipi_id);
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

pub fn gic_is_priv(int_id: usize) -> bool {
    int_id < GIC_PRIVATE_INT_NUM
}

pub fn gic_lrs() -> usize {
    *GIC_LRS_NUM.lock()
}

pub fn set_gic_lrs(lrs: usize) {
    let mut gic_lrs = GIC_LRS_NUM.lock();
    *gic_lrs = lrs;
}

#[cfg(feature = "hv")]
pub fn gicc_clear_current_irq(irq:usize, for_hypervisor: bool) {
    //debug!("gicc_clear_current_irq: irq {}, for_hypervisor {}", irq, for_hypervisor);
    if irq == 0 {
        return;
    }
    GICC.set_eoi(irq as _);
    if for_hypervisor {
        // let addr = 0x08010000 + 0x1000;
        // unsafe {
        //     let gicc_dir = addr as *mut u32;
        //     *gicc_dir = irq;
        // }
        GICC.set_dir(irq as _);
    }
}

#[cfg(feature = "hv")]
pub fn gicv_clear_current_irq(irq:usize, for_hypervisor: bool) {
    debug!("gicv_clear_current_irq: irq {}, for_hypervisor {}", irq, for_hypervisor);
    if irq == 0 {
        return;
    }
    GICV.set_eoi(irq as _);

    if for_hypervisor {
        // let addr = 0x08010000 + 0x1000;
        // unsafe {
        //     let gicc_dir = addr as *mut u32;
        //     *gicc_dir = irq;
        // }
        GICV.set_dir(irq as _);
    }
}