

use arm_gicv3::gich_lrs_num;
pub use arm_gicv3::GICD;
pub use arm_gicv3::GICR;
pub use arm_gicv3::GICC;
pub use arm_gicv3::GICH;
pub use arm_gicv3::{ GICC_IAR_ID_OFF, GICC_IAR_ID_LEN, GIC_SPI_MAX, GIC_SGIS_NUM };
use arm_gicv3::gic_is_priv;
use arm_gicv3::bit_extract;
use arm_gicv3::GIC_LRS_NUM;

pub use arm_gicv3::GIC_INTS_MAX as MAX_IRQ_COUNT;
use arm_gic::translate_irq;
use arm_gic::InterruptType;

use crate::cpu::this_cpu_id;

use core::sync::atomic::Ordering;

use crate::irq::IrqHandler;

// like gic v2
pub const IPI_IRQ_NUM:               usize = 1;
pub const HYPERVISOR_TIMER_IRQ_NUM:  usize = 26;
pub const MAINTENANCE_IRQ_NUM:       usize = 25;

// pub const UART_IRQ_NUM: usize = translate_irq(axconfig::UART_IRQ, InterruptType::SPI).unwrap();

// 来自v2
/// The timer IRQ number.
pub const TIMER_IRQ_NUM: usize = 30; // physical timer, type=PPI, id=14

/* =========================================== */
/* ================ InterFace ================ */
/* =========================================== */

/// Initializes GICD, GICC on the primary CPU.
pub(crate) fn init_primary() {
    gic_glb_init();
    gic_cpu_init();
}

/// Initializes GICC on secondary CPUs.
#[cfg(feature = "smp")] pub(crate) fn init_secondary() {
    info!("Initialize init_secondary GICv3...");
    // GICC.init();
    gic_cpu_init();
}

/// Enables or disables the given IRQ.
pub fn set_enable(irq_num: usize, enabled: bool) {
    use tock_registers::interfaces::Readable;
    gic_set_enable(irq_num, enabled);
    gic_set_prio(irq_num, 0x1);
    GICD.set_route(irq_num, cortex_a::registers::MPIDR_EL1.get() as usize);
}

/// Registers an IRQ handler for the given IRQ.
///
/// It also enables the IRQ if the registration succeeds. It returns `false` if
/// the registration failed.
pub fn register_handler(irq_num: usize, handler: IrqHandler) -> bool {
    // 这里v3的处理应该和v2相同，不会出问题
    crate::irq::register_handler_common(irq_num, handler)
}

#[cfg(feature = "hv")]
pub fn dispatch_irq(irq_num: usize) {
    debug!("dispatch_irq_hv: irq_num {}", irq_num);
    crate::irq::dispatch_irq_common(irq_num as _);
}

#[cfg(not(feature = "hv"))]
pub fn dispatch_irq(_irq_num: usize) {
    let intid = GICC.iar() as usize;
    crate::irq::dispatch_irq_common(intid);
    GICC.set_eoir(0);
}



// Isn't right ?
pub fn interrupt_cpu_ipi_send(cpu_id: usize, ipi_id: usize) {
    debug!("interrupt_cpu_ipi_send: cpu_id {}, ipi_id {}", cpu_id, ipi_id);
    if ipi_id < GIC_SGIS_NUM {
       GICD.send_sgi(cpu_id, ipi_id);
    }
}

// Isn't right ?
pub fn deactivate_irq(iar: usize) {
    GICC.set_eoir(iar as _);    
}



/* =============== warn! ===================== 
   ========   current_cpu().current_irq   ====
   ===========================================
*/
// pub fn gicc_clear_current_irq(for_hypervisor: bool) {
//     let irq = current_cpu().current_irq as u32;
//     if irq == 0 {
//         return;
//     }
//     GICC.set_eoir(irq);
//     if for_hypervisor {
//         GICC.set_dir(irq);
//     }
//     current_cpu().current_irq = 0;
// }

// pub fn gicc_get_current_irq() -> Option<usize> {
//     let iar = GICC.iar();
//     let irq = iar as usize;
//     current_cpu().current_irq = irq;
//     let id = bit_extract(iar as usize, GICC_IAR_ID_OFF, GICC_IAR_ID_LEN);
//     if id >= 1024 {   // IntCtrl::NUM_MAX
//         None
//     } else {
//         Some(id)
//     }
// }


// Isn't right ?
// remove current_cpu().current_irq, return (usize, usize)
pub fn gicc_get_current_irq() -> (usize, usize) {
    let iar = GICC.iar();
    let irq = iar as usize;
    debug!("this is iar:{:#x}", iar);
    // current_cpu().current_irq = irq;
    let irq = bit_extract(irq, 0, 10);
    let src = bit_extract(irq, 10, 3);
    (irq, src)
}

// Isn't right ?
// remove current_cpu().current_irq ,add an argument
pub fn gicc_clear_current_irq( irq: usize, for_hypervisor: bool) {
    // let irq = current_cpu().current_irq as u32;
    debug!("gicc clear current irq: {}", irq);
    let irq = irq as u32;
    if irq == 0 {
        return;
    }
    GICC.set_eoir(irq);
    if for_hypervisor {
        GICC.set_dir(irq);
    }
    // current_cpu().current_irq = 0;
}


/* =========================================== */
/* ================ InterFace ================ */
/* =========================================== */


pub fn gic_glb_init() {
    debug!("====== glb_init start ======");
    let gich_num = gich_lrs_num();
    debug!("file: gicv3, gich_num: {}", gich_num);
    set_gic_lrs(gich_num);
    debug!("file: gicv3, GIC_LRS_NUM store: {}", gic_lrs());
    GICD.global_init();
    debug!("====== glb_init ended ======");
}

pub fn gic_cpu_init() {
    let cpu_id = this_cpu_id();
    debug!("this cpu id {}", cpu_id);
    GICR.init(cpu_id);
    GICC.init();
}

pub fn gic_cpu_reset() {
    GICC.init();
}

pub fn gic_set_icfgr(int_id: usize, cfg: u8) {
    if !gic_is_priv(int_id) {
        GICD.set_icfgr(int_id, cfg);
    } else {
        GICR.set_icfgr(int_id, cfg, this_cpu_id() as u32);
    }
}

pub fn gic_lrs() -> usize {
    GIC_LRS_NUM.load(Ordering::Relaxed)
}

pub fn set_gic_lrs(lrs: usize) {
    GIC_LRS_NUM.store(lrs, Ordering::Relaxed);
}

pub fn gic_set_act(int_id: usize, act: bool, gicr_id: u32) {
    if !gic_is_priv(int_id) {
        GICD.set_act(int_id, act);
    } else {
        GICR.set_act(int_id, act, gicr_id);
    }
}

pub fn gic_set_pend(int_id: usize, pend: bool, gicr_id: u32) {
    if !gic_is_priv(int_id) {
        GICD.set_pend(int_id, pend);
    } else {
        GICR.set_pend(int_id, pend, gicr_id);
    }
}

pub fn gic_get_pend(int_id: usize) -> bool {
    if !gic_is_priv(int_id) {
        GICD.get_pend(int_id)
    } else {
        GICR.get_pend(int_id, this_cpu_id() as u32)
    }
}

pub fn gic_get_act(int_id: usize) -> bool {
    if !gic_is_priv(int_id) {
        GICD.get_act(int_id)
    } else {
        GICR.get_act(int_id, this_cpu_id() as u32)
    }
}


// 一下三个函数在开启或关闭某个中断使用 set_enable

pub fn gic_set_enable(int_id: usize, en: bool) {
    if !gic_is_priv(int_id) {
        GICD.set_enable(int_id, en);
    } else {
        GICR.set_enable(int_id, en, this_cpu_id() as u32);
    }
}

pub fn gic_get_prio(int_id: usize) {
    if !gic_is_priv(int_id) {
        GICD.prio(int_id);
    } else {
        GICR.get_prio(int_id, this_cpu_id() as u32);
    }
}

pub fn gic_set_prio(int_id: usize, prio: u8) {
    if !gic_is_priv(int_id) {
        GICD.set_prio(int_id, prio);
    } else {
        GICR.set_prio(int_id, prio, this_cpu_id() as u32);
    }
}
