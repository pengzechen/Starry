use crate::{irq::IrqHandler, mem::phys_to_virt};
use arm_gic::{translate_irq, GenericArmGic, IntId, InterruptType};
use memory_addr::PhysAddr;
use spinlock::SpinNoIrq;

/// The maximum number of IRQs.
pub const MAX_IRQ_COUNT: usize = IntId::GIC_MAX_IRQ;
/// The timer IRQ number.
pub const TIMER_IRQ_NUM: usize = translate_irq(14, InterruptType::PPI).unwrap();

/// The UART IRQ number.
pub const UART_IRQ_NUM: usize = translate_irq(axconfig::UART_IRQ, InterruptType::SPI).unwrap();

pub const GICD_BASE: PhysAddr = PhysAddr::from(axconfig::GICD_PADDR);

const GICC_BASE: PhysAddr = PhysAddr::from(axconfig::GICC_PADDR);

/* HV start */
use arm_gic::{GIC_SGIS_NUM, GIC_PRIVATE_INT_NUM};
pub const GIC_SPI_MAX: usize = MAX_IRQ_COUNT - GIC_PRIVATE_INT_NUM;
pub const IPI_IRQ_NUM: usize = 1;

#[cfg(feature = "hv")]
/// The maintenance interrupt irq number.
pub const MAINTENANCE_IRQ_NUM: usize = 25;

#[cfg(feature = "hv")]
/// The hypervisor timer irq number.
pub const HYPERVISOR_TIMER_IRQ_NUM: usize = 26;

use arm_gic::GicHypervisorInterface;
use arm_gic::gic_v2::GicVcpuInterface;
use spin::Mutex;

// 需要确定位置  GICH_PADDR
const GICH_BASE: PhysAddr = PhysAddr::from(0x0803_0000);
// 需要确定位置  GICH_PADDR
const GICV_BASE: PhysAddr = PhysAddr::from(0x0804_0000);

pub static GICH: GicHypervisorInterface = GicHypervisorInterface::new(phys_to_virt(GICH_BASE).as_mut_ptr());

pub static GICV: GicVcpuInterface = GicVcpuInterface::new(phys_to_virt(GICV_BASE).as_mut_ptr());

pub static GIC_LRS_NUM: Mutex<usize> = Mutex::new(0);


/* HV end */

cfg_if::cfg_if! {
    if #[cfg(platform_family= "aarch64-rk3588j")] {
        use arm_gic::GicV3;
        pub static mut GIC: SpinNoIrq<GicV3> =
            SpinNoIrq::new(GicV3::new(phys_to_virt(GICD_BASE).as_mut_ptr(), phys_to_virt(GICC_BASE).as_mut_ptr()));
    } else {
        use arm_gic::GicV2;
        pub static mut GIC: SpinNoIrq<GicV2> =
            SpinNoIrq::new(GicV2::new(phys_to_virt(GICD_BASE).as_mut_ptr(), phys_to_virt(GICC_BASE).as_mut_ptr()));
    }
}

/// Enables or disables the given IRQ.
pub fn set_enable(irq_num: usize, enabled: bool) {
    trace!("GICD set enable: {} {}", irq_num, enabled);

    // SAFETY:
    // access percpu interface through get_mut, no need to lock
    // it will introduce side effects: need to add unsafe
    // Acceptable compared to data competition
    unsafe {
        if enabled {
            GIC.lock().enable_interrupt(irq_num.into());
        } else {
            GIC.lock().disable_interrupt(irq_num.into());
        }
    }
}

/// Registers an IRQ handler for the given IRQ.
///
/// It also enables the IRQ if the registration succeeds. It returns `false` if
/// the registration failed.
pub fn register_handler(irq_num: usize, handler: IrqHandler) -> bool {
    trace!("register handler irq {}", irq_num);
    crate::irq::register_handler_common(irq_num, handler)
}

/// Dispatches the IRQ.
///
/// This function is called by the common interrupt handler. It looks
/// up in the IRQ handler table and calls the corresponding handler. If
/// necessary, it also acknowledges the interrupt controller after handling.
pub fn dispatch_irq(_unused: usize) {
    // actually no need to lock
    let intid = unsafe { GIC.get_mut().get_and_acknowledge_interrupt() };
    if let Some(id) = intid {
        crate::irq::dispatch_irq_common(id.into());
        unsafe {
            GIC.get_mut().end_interrupt(id);
        }
    }
}

/// Initializes GICD, GICC on the primary CPU.
pub(crate) fn init_primary() {
    info!("Initialize GICv2...");
    unsafe { GIC.lock().init_primary() };
}

/// Initializes GICC on secondary CPUs.
#[cfg(feature = "smp")]
pub(crate) fn init_secondary() {
    // per cpu handle, no need lock
    unsafe { GIC.get_mut().per_cpu_init() };
}


/* HV start */
use hypercraft::arch::utils::bit_extract;

pub fn gicc_get_current_irq() -> (usize, usize) {
    let iar;
    unsafe { iar = GIC.lock().get_iar(); }
    let irq = iar as usize;
    // debug!("this is iar:{:#x}", iar);
    // current_cpu().current_irq = irq;
    let irq = bit_extract(irq, 0, 10);
    let src = bit_extract(irq, 10, 3);
    (irq, src)
}

#[no_mangle]
pub fn interrupt_cpu_ipi_send(cpu_id: usize, ipi_id: usize) {
    debug!("interrupt_cpu_ipi_send: cpu_id {}, ipi_id {}", cpu_id, ipi_id);
    if ipi_id < GIC_SGIS_NUM {
       unsafe{ GIC.lock().send_sgi(cpu_id, ipi_id); }
    }
}

pub fn deactivate_irq(iar: usize) {
    unsafe{ GIC.lock().set_eoi(iar as _); }    
}

pub fn gicc_clear_current_irq(irq:usize, for_hypervisor: bool) {
    debug!("gicc_clear_current_irq: irq {}, for_hypervisor {}", irq, for_hypervisor);
    if irq == 0 {
        return;
    }
    unsafe{ GIC.lock().set_eoi(irq as _); }
    if for_hypervisor {
        // let addr = 0x08010000 + 0x1000;
        // unsafe {
        //     let gicc_dir = addr as *mut u32;
        //     *gicc_dir = irq;
        // }
        unsafe{ GIC.lock().set_dir(irq as _); }
    }
}

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

pub fn gic_is_priv(int_id: usize) -> bool {
    int_id < GIC_PRIVATE_INT_NUM
}

pub fn gic_lrs() -> usize {
    *GIC_LRS_NUM.lock()
}


/* HV end */