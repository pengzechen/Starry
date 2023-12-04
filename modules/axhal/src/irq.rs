//! Interrupt management.

use handler_table::HandlerTable;

use crate::platform::irq::MAX_IRQ_COUNT;

pub use crate::platform::irq::{dispatch_irq, register_handler, set_enable};

/// The type if an IRQ handler.
pub type IrqHandler = handler_table::Handler;

static IRQ_HANDLER_TABLE: HandlerTable<MAX_IRQ_COUNT> = HandlerTable::new();

/// Platform-independent IRQ dispatching.
#[allow(dead_code)]
pub(crate) fn dispatch_irq_common(irq_num: usize) {
    trace!("IRQ {}", irq_num);
    if !IRQ_HANDLER_TABLE.handle(irq_num) {
        warn!("Unhandled IRQ {}", irq_num);
    }
}

/// Platform-independent IRQ handler registration.
///
/// It also enables the IRQ if the registration succeeds. It returns `false` if
/// the registration failed.
#[allow(dead_code)]
pub(crate) fn register_handler_common(irq_num: usize, handler: IrqHandler) -> bool {
    if irq_num < MAX_IRQ_COUNT && IRQ_HANDLER_TABLE.register_handler(irq_num, handler) {
        debug!("!!!!!!!!!!!!!!register handler for IRQ {} success", irq_num);
        set_enable(irq_num, true);
        return true;
    }
    warn!("register handler for IRQ {} failed", irq_num);
    false
}

#[allow(dead_code)]
pub fn irq_num_exist(irq_num: usize) -> bool {
    IRQ_HANDLER_TABLE.irq_num_exist(irq_num)
}
/* 
#[cfg(all(feature = "hv", target_arch = "aarch64"))]
static IRQ_HANDLER_TABLE_EL2: HandlerTable<MAX_IRQ_COUNT> = HandlerTable::new();

#[cfg(all(feature = "hv", target_arch = "aarch64"))]
#[allow(dead_code)]
pub(crate) fn register_handler_el2(irq_num: usize, handler: IrqHandler) -> bool {
    if irq_num < MAX_IRQ_COUNT && IRQ_HANDLER_TABLE_EL2.register_handler(irq_num, handler) {
        set_enable(irq_num, true);
        return true;
    }
    warn!("register handler for IRQ {} failed", irq_num);
    false
}
*/