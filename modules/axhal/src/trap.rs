//! Trap handling.

use crate_interface::{call_interface, def_interface};

/// Trap handler interface.
///
/// This trait is defined with the [`#[def_interface]`][1] attribute. Users
/// should implement it with [`#[impl_interface]`][2] in any other crate.
///
/// [1]: crate_interface::def_interface
/// [2]: crate_interface::impl_interface
#[def_interface]
pub trait TrapHandler {
    /// Handles interrupt requests for the given IRQ number.
    fn handle_irq(irq_num: usize);
    #[cfg(all(feature = "hv", target_arch = "aarch64"))]
    /// Handles interrupt requests for the given IRQ number for route to el2.
    fn handle_irq_hv(irq_num: usize, src: usize);
    // more e.g.: handle_page_fault();
}

/// Call the external IRQ handler.
#[allow(dead_code)]
pub(crate) fn handle_irq_extern(irq_num: usize) {
    call_interface!(TrapHandler::handle_irq, irq_num);
}

/// Call the external IRQ handler.
#[allow(dead_code)]
#[cfg(all(feature = "hv", target_arch = "aarch64"))]
pub(crate) fn handle_irq_extern_hv(irq_num: usize, src: usize) {
    debug!("in handle_irq_extern_hv: irq_num {}, src {}", irq_num, src);
    call_interface!(TrapHandler::handle_irq_hv, irq_num, src);
}