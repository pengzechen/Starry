mod boot;

#[cfg(all(feature = "irq", not(feature = "hv")))]
pub mod generic_timer;

#[cfg(all(feature = "irq", feature = "hv"))]
pub mod generic_timer_hv;

pub mod dw_apb_uart;
pub mod pl011;
pub mod psci;


#[cfg(all(feature = "irq", not(feature = "gic_v3")))]
pub mod gic;

#[cfg(all(feature = "irq", feature = "gic_v3"))]
pub mod gicv3;

