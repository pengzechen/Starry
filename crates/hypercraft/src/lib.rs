//! HyperCraft is a VMM crate.

#![no_std]
#![allow(
    clippy::upper_case_acronyms,
    clippy::single_component_path_imports,
    clippy::collapsible_match,
    clippy::default_constructed_unit_structs,
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    unused_imports,
    unused_assignments
)]
// #![deny(missing_docs, warnings)]

#![feature(naked_functions, asm_const, negative_impls, stdsimd, inline_const, concat_idents)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;

#[path = "arch/aarch64/mod.rs"]
/// Aarch64 arch code.
pub mod arch;


mod hal;
mod memory;
mod traits;
mod vcpus;

/// HyperCraft Result Define.
pub type HyperResult<T = ()> = Result<T, HyperError>;


pub use arch::{
    NestedPageTable, PerCpu, VCpu, VM,
};

pub use hal::HyperCraftHal;
pub use memory::{
    GuestPageNum, GuestPageTableTrait, GuestPhysAddr, GuestVirtAddr, HostPageNum, HostPhysAddr,
    HostVirtAddr,
};
pub use vcpus::VmCpus;

pub use arch::VcpusArray;
#[cfg(not(feature = "gic_v3"))]
pub use arch::gic::IrqState;
#[cfg(not(feature = "gic_v3"))]
pub use arch::gic;

#[cfg(feature = "gic_v3")]
pub use arch::gicv3::IrqState;
#[cfg(feature = "gic_v3")]
pub use arch::gicv3;

/// The error type for hypervisor operation failures.
#[derive(Debug, PartialEq)]
pub enum HyperError {
    /// Internal error.
    Internal,
    /// No supported error.
    NotSupported,
    /// No memory error.
    NoMemory,
    /// Invalid parameter error.
    InvalidParam,
    /// Invalid instruction error.
    InvalidInstruction,
    /// Memory out of range error.
    OutOfRange,
    /// Bad state error.
    BadState,
    /// Not found error.
    NotFound,
    /// Fetch instruction error.
    FetchFault,
    /// Page fault error.
    PageFault,
    /// Decode error.
    DecodeError,
    /// Disabled.
    Disabled,
}
