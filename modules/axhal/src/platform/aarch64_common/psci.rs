#![allow(dead_code)]

//! ARM Power State Coordination Interface.

use core::arch::asm;

#[cfg(not(feature = "hv"))]
const PSCI_CPU_ON: u32 = 0x8400_0003;
#[cfg(not(feature = "hv"))]
const PSCI_SYSTEM_OFF: u32 = 0x8400_0008;

#[cfg(feature = "hv")]
const PSCI_CPU_ON: u32 = 0xc400_0003;
#[cfg(feature = "hv")]
const PSCI_SYSTEM_OFF: u32 = 0x8400_0008;

#[cfg(not(feature = "hv"))]
fn psci_hvc_call(func: u32, arg0: usize, arg1: usize, arg2: usize) -> usize {
    debug!("this is hvc call func:0x{:x}", func);
    let ret;
    unsafe {
        asm!(
            "hvc #0",
            inlateout("x0") func as usize => ret,
            in("x1") arg0,
            in("x2") arg1,
            in("x3") arg2,
        )
    }
    ret
}

#[cfg(feature = "hv")]
fn psci_smc_call(func: u32, arg0: usize, arg1: usize, arg2: usize) -> usize {
    debug!("this is smc call func:0x{:x}", func);
    let mut ret = 0;
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!(
            "smc #0",
            inlateout("x0") func as usize => ret,
            in("x1") arg0,
            in("x2") arg1,
            in("x3") arg2,
            options(nomem, nostack)
        );
    }
    ret
}

/// Shutdown the whole system, including all CPUs.
pub fn system_off() -> ! {
    info!("Shutting down...");
    #[cfg(not(feature = "hv"))]
    psci_hvc_call(PSCI_SYSTEM_OFF, 0, 0, 0);
    #[cfg(feature = "hv")]
    psci_smc_call(PSCI_SYSTEM_OFF, 0, 0, 0);
    warn!("It should shutdown!");
    loop {
        crate::arch::halt();
    }
}

/// Starts a secondary CPU with the given ID.
///
/// When the CPU is started, it will jump to the given entry and set the
/// corresponding register to the given argument.
pub fn cpu_on(id: usize, entry: usize, arg: usize) {
    debug!("Starting core {}...", id);
    #[cfg(not(feature = "hv"))]
    assert_eq!(psci_hvc_call(PSCI_CPU_ON, id, entry, arg), 0);
    #[cfg(feature = "hv")]
    assert_eq!(psci_smc_call(PSCI_CPU_ON, id, entry, arg), 0);
}
