mod context_frame;
mod cpu;
mod vm;
mod gic;
mod ept;
mod vcpus_array;

/// hypervisor call hvc mod
pub mod hvc;
/// virtual cpu mod
pub mod vcpu;
/// utils for aarch64
pub mod utils;
/// vitual gic
pub mod vgic;
/// virtual uart
pub mod vuart;

/// emulate device
pub mod emu;

// pub use gic::{GICC, GICD, GICH, GICD_BASE};
pub use ept::NestedPageTable;
pub use vcpu::VCpu;
pub use vm::VM;
pub use cpu::PerCpu;
pub use vcpus_array::VcpusArray;

pub use page_table::PageSize;
pub use gic::IrqState;

/// context frame for aarch64
pub type ContextFrame = crate::arch::context_frame::Aarch64ContextFrame;

/// Move to ARM register from system coprocessor register.
/// MRS Xd, sysreg "Xd = sysreg"
#[macro_export]
macro_rules! mrs {
    ($val: expr, $reg: expr, $asm_width:tt) => {
        unsafe {
            core::arch::asm!(concat!("mrs {0:", $asm_width, "}, ", stringify!($reg)), out(reg) $val, options(nomem, nostack));
        }
    };
    ($val: expr, $reg: expr) => {
        unsafe {
            core::arch::asm!(concat!("mrs {0}, ", stringify!($reg)), out(reg) $val, options(nomem, nostack));
        }
    };
}

/// Move to system coprocessor register from ARM register.
/// MSR sysreg, Xn "sysreg = Xn"
#[macro_export]
macro_rules! msr {
    ($reg: expr, $val: expr, $asm_width:tt) => {
        unsafe {
            core::arch::asm!(concat!("msr ", stringify!($reg), ", {0:", $asm_width, "}"), in(reg) $val, options(nomem, nostack));
        }
    };
    ($reg: expr, $val: expr) => {
        unsafe {
            core::arch::asm!(concat!("msr ", stringify!($reg), ", {0}"), in(reg) $val, options(nomem, nostack));
        }
    };
}

/// aarch64 context frame trait
pub trait ContextFrameTrait {
    /// create a new context frame
    fn new(pc: usize, sp: usize, arg: usize) -> Self;
    /// get the exception program counter
    fn exception_pc(&self) -> usize;
    /// set the exception program counter
    fn set_exception_pc(&mut self, pc: usize);
    /// get the stack pointer
    fn stack_pointer(&self) -> usize;
    /// set the stack pointer
    fn set_stack_pointer(&mut self, sp: usize);
    /// get the argument (register x0)
    fn set_argument(&mut self, arg: usize);
    /// set gpr by idx
    fn set_gpr(&mut self, index: usize, val: usize);
    /// get gpr by idx
    fn gpr(&self, index: usize) -> usize;
}

