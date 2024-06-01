//! Platform-specific operations.


pub(crate) mod aarch64_common;
pub use self::aarch64_common::*;


// rk3588 这个地方暂时使用 qemu-virt 
mod qemu_virt_aarch64;
pub use self::qemu_virt_aarch64::*;
    
