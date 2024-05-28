#![allow(unused_imports)]

use aarch64_cpu::registers::{CNTFRQ_EL0, CNTVCT_EL0, CNTV_CTL_EL0, CNTV_TVAL_EL0};
use ratio::Ratio;
use tock_registers::interfaces::{Readable, Writeable};

static mut CNTVCT_TO_NANOS_RATIO: Ratio = Ratio::zero();
static mut NANOS_TO_CNTVCT_RATIO: Ratio = Ratio::zero();

use hypercraft::msr;

/// Returns the current clock time in hardware ticks.
#[inline]
pub fn current_ticks() -> u64 {
    CNTVCT_EL0.get()
}

/// Converts hardware ticks to nanoseconds.
#[inline]
pub fn ticks_to_nanos(ticks: u64) -> u64 {
    unsafe { CNTVCT_TO_NANOS_RATIO.mul_trunc(ticks) }
}

/// Converts nanoseconds to hardware ticks.
#[inline]
pub fn nanos_to_ticks(nanos: u64) -> u64 {
    unsafe { NANOS_TO_CNTVCT_RATIO.mul_trunc(nanos) }
}

/// Early stage initialization: stores the timer frequency.
pub(crate) fn init_early() {
    let freq = CNTFRQ_EL0.get();
    unsafe {
        CNTVCT_TO_NANOS_RATIO = Ratio::new(crate::time::NANOS_PER_SEC as u32, freq as u32);
        NANOS_TO_CNTVCT_RATIO = CNTVCT_TO_NANOS_RATIO.inverse();
    }
}


pub fn set_oneshot_timer(deadline_ns: u64) {
    let cnptct = CNTVCT_EL0.get();
    let cnptct_deadline = nanos_to_ticks(deadline_ns);
    if cnptct < cnptct_deadline {
        let interval = cnptct_deadline - cnptct;
        debug_assert!(interval <= u32::MAX as u64);
        msr!(CNTHP_TVAL_EL2, interval as u64);
    } else {
        msr!(CNTHP_TVAL_EL2, 0);
    }
}


pub(crate) fn init_percpu() {
    let ctl = 1;
    let tval = 0;
    msr!(CNTHP_CTL_EL2, ctl);
    msr!(CNTHP_TVAL_EL2, tval);
    // crate::platform::irq::set_enable(crate::platform::irq::HYPERVISOR_TIMER_IRQ_NUM, true);
}