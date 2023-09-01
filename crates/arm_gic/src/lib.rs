//! ARM Generic Interrupt Controller (GIC) register definitions and basic
//! operations.

#![no_std]
#![feature(const_ptr_as_ref)]
#![feature(const_option)]
#![feature(const_nonnull_new)]

pub mod gic_v2;

use core::ops::Range;

/// Interrupt ID 0-15 are used for SGIs (Software-generated interrupt).
///
/// SGI is an interrupt generated by software writing to a GICD_SGIR register in
/// the GIC. The system uses SGIs for interprocessor communication.
pub const SGI_RANGE: Range<usize> = 0..16;

/// Interrupt ID 16-31 are used for PPIs (Private Peripheral Interrupt).
///
/// PPI is a peripheral interrupt that is specific to a single processor.
pub const PPI_RANGE: Range<usize> = 16..32;

/// Interrupt ID 32-1019 are used for SPIs (Shared Peripheral Interrupt).
///
/// SPI is a peripheral interrupt that the Distributor can route to any of a
/// specified combination of processors.
pub const SPI_RANGE: Range<usize> = 32..1020;

/// Maximum number of interrupts supported by the GIC.
pub const GIC_MAX_IRQ: usize = 1024;

/// Interrupt trigger mode.
pub enum TriggerMode {
    /// Edge-triggered.
    ///
    /// This is an interrupt that is asserted on detection of a rising edge of
    /// an interrupt signal and then, regardless of the state of the signal,
    /// remains asserted until it is cleared by the conditions defined by this
    /// specification.
    Edge = 0,
    /// Level-sensitive.
    ///
    /// This is an interrupt that is asserted whenever the interrupt signal
    /// level is active, and deasserted whenever the level is not active.
    Level = 1,
}

//#[cfg(feature = "hv")]
pub const GIC_LIST_REGS_NUM: usize = 64;