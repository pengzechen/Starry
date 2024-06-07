#![allow(unused_imports)]

use aarch64_cpu::registers::{CNTFRQ_EL0, CNTPCT_EL0, CNTP_CTL_EL0, CNTP_TVAL_EL0};
use ratio::Ratio;
use tock_registers::interfaces::{Readable, Writeable};

static mut CNTPCT_TO_NANOS_RATIO: Ratio = Ratio::zero();
static mut NANOS_TO_CNTPCT_RATIO: Ratio = Ratio::zero();

/// Returns the current clock time in hardware ticks.
#[inline]
pub fn current_ticks() -> u64 {
    CNTPCT_EL0.get()
}

/// Converts hardware ticks to nanoseconds.
#[inline]
pub fn ticks_to_nanos(ticks: u64) -> u64 {
    unsafe { CNTPCT_TO_NANOS_RATIO.mul_trunc(ticks) }
}

/// Converts nanoseconds to hardware ticks.
#[inline]
pub fn nanos_to_ticks(nanos: u64) -> u64 {
    unsafe { NANOS_TO_CNTPCT_RATIO.mul_trunc(nanos) }
}


/// Early stage initialization: stores the timer frequency.
pub(crate) fn init_early() {
    let freq = CNTFRQ_EL0.get();
    unsafe {
        CNTPCT_TO_NANOS_RATIO = Ratio::new(crate::time::NANOS_PER_SEC as u32, freq as u32);
        NANOS_TO_CNTPCT_RATIO = CNTPCT_TO_NANOS_RATIO.inverse();
    }
}



/// Set a one-shot timer.
///
/// A timer interrupt will be triggered at the given deadline (in nanoseconds).
// xh not sure
#[cfg(all(feature = "irq", feature = "hv"))]
pub fn set_oneshot_timer(deadline_ns: u64) {
    let cnptct = CNTPCT_EL0.get();
    let cnptct_deadline = nanos_to_ticks(deadline_ns);
    if cnptct < cnptct_deadline {
        let interval = cnptct_deadline - cnptct;
        debug_assert!(interval <= u32::MAX as u64);
        CNTP_TVAL_EL0.set(interval);
    } else {
        CNTP_TVAL_EL0.set(0);
    }
}

// xh not sure
#[cfg(all(feature = "irq", not(feature = "hv")))]
pub fn set_oneshot_timer(deadline_ns: u64) {
    let cnptct = CNTPCT_EL0.get();
    let cnptct_deadline = nanos_to_ticks(deadline_ns);
    if cnptct < cnptct_deadline {
        let interval = cnptct_deadline - cnptct;
        debug_assert!(interval <= u32::MAX as u64);
        msr!(CNTHP_TVAL_EL2, interval as u64);
    } else {
        msr!(CNTHP_TVAL_EL2, 0);
    }
}

/// Early stage initialization: stores the timer frequency.

use hypercraft::msr;
pub(crate) fn init_percpu() {
    #[cfg(all(feature = "irq", not(feature = "hv")))]
    {
        CNTP_CTL_EL0.write(CNTP_CTL_EL0::ENABLE::SET);
        CNTP_TVAL_EL0.set(0);
        crate::platform::irq::set_enable(crate::platform::irq::TIMER_IRQ_NUM, true);
    }
    #[cfg(feature = "hv")]
    {
        let ctl = 1;
        let tval = 0;
        msr!(CNTHP_CTL_EL2, ctl);
        msr!(CNTHP_TVAL_EL2, tval);
        crate::platform::irq::set_enable(crate::platform::gicv3::HYPERVISOR_TIMER_IRQ_NUM, true);
    }
}