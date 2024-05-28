pub mod mem;

#[cfg(feature = "smp")]
pub mod mp;

#[cfg(all(feature = "irq", not(feature = "gic_v3")))]
pub mod irq {
    pub use crate::platform::aarch64_common::gic::*;
}

#[cfg(all(feature = "irq", feature = "gic_v3"))]
pub mod irq {
    pub use crate::platform::aarch64_common::gicv3::*;
}

pub mod console {
    pub use crate::platform::aarch64_common::pl011::*;
}

pub mod time {
    
    #[cfg(all(feature = "irq", not(feature = "hv")))]
    pub use crate::platform::aarch64_common::generic_timer::*;

    #[cfg(all(feature = "irq", feature = "hv"))]
    pub use crate::platform::aarch64_common::generic_timer_hv::*;
}

pub mod misc {
    pub use crate::platform::aarch64_common::psci::system_off as terminate;
}

extern "C" {
    fn exception_vector_base();
    fn rust_main(cpu_id: usize, dtb: usize);
    #[cfg(feature = "smp")]
    fn rust_main_secondary(cpu_id: usize);
}

pub(crate) unsafe extern "C" fn rust_entry(cpu_id: usize, dtb: usize) {
    crate::mem::clear_bss();
    crate::arch::set_exception_vector_base(exception_vector_base as usize);
    crate::cpu::init_primary(cpu_id);
    super::aarch64_common::pl011::init_early();             // 初始化锁
    super::aarch64_common::generic_timer_hv::init_early();     // 读寄存器 初始化频率  // 临时添加
    rust_main(cpu_id, dtb);
}

#[cfg(feature = "smp")]
pub(crate) unsafe extern "C" fn rust_entry_secondary(cpu_id: usize) {
    crate::arch::set_exception_vector_base(exception_vector_base as usize);
    crate::cpu::init_secondary(cpu_id);
    rust_main_secondary(cpu_id);
}

/// Initializes the platform devices for the primary CPU.
///
/// For example, the interrupt controller and the timer.
pub fn platform_init() {
    #[cfg(all(feature = "irq", not(feature = "gic_v3")))]
    super::aarch64_common::gic::init_primary();
    #[cfg(all(feature = "irq", feature = "gic_v3"))]
    super::aarch64_common::gicv3::init_primary();
    super::aarch64_common::generic_timer_hv::init_percpu();                             // 临时添加
    super::aarch64_common::pl011::init();
}

/// Initializes the platform devices for secondary CPUs.
#[cfg(feature = "smp")]
pub fn platform_init_secondary() {
    #[cfg(feature = "irq")]
    super::aarch64_common::gic::init_secondary();
    super::aarch64_common::generic_timer::init_percpu();
}
