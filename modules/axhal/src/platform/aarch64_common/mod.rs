mod boot;

pub mod generic_timer;
pub mod pl011;
pub mod psci;


#[cfg(all(feature = "irq", not(feature = "gic_v3")))]
pub mod gic;

#[cfg(all(feature = "irq", feature = "gic_v3"))]
pub mod gicv3;

