#![allow(unused_imports)]

use aarch64_cpu::registers::{
    CNTFRQ_EL0, 
    CNTPCT_EL0, 
    CNTP_CTL_EL0, 
    CNTP_TVAL_EL0
};
use tock_registers::interfaces::{
    Readable, 
    Writeable
};
use ratio::Ratio;


static mut CNTPCT_TO_NANOS_RATIO: Ratio = Ratio::zero();
static mut NANOS_TO_CNTPCT_RATIO: Ratio = Ratio::zero();


#[inline] /// Returns the current clock time in hardware ticks.
pub fn current_ticks() -> u64 {
    CNTPCT_EL0.get()
}

#[inline] /// Converts hardware ticks to nanoseconds.
pub fn ticks_to_nanos(ticks: u64) -> u64 {
    unsafe { CNTPCT_TO_NANOS_RATIO.mul_trunc(ticks) }
}

#[inline] /// Converts nanoseconds to hardware ticks.
pub fn nanos_to_ticks(nanos: u64) -> u64 {
    unsafe { NANOS_TO_CNTPCT_RATIO.mul_trunc(nanos) }
}

#[cfg(feature = "irq")] /// A timer interrupt will be triggered at the given deadline (in nanoseconds). /// Set a one-shot timer.
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

/// Early stage initialization: stores the timer frequency.
pub(crate) fn init_early() {
    let freq = CNTFRQ_EL0.get();
    unsafe {
        CNTPCT_TO_NANOS_RATIO = Ratio::new(crate::time::NANOS_PER_SEC as u32, freq as u32);
        NANOS_TO_CNTPCT_RATIO = CNTPCT_TO_NANOS_RATIO.inverse();
    }
}

pub(crate) fn init_percpu() {
    #[cfg(feature = "irq")]
    {
        CNTP_CTL_EL0.write(CNTP_CTL_EL0::ENABLE::SET);
        CNTP_TVAL_EL0.set(0);
        crate::platform::irq::set_enable(crate::platform::irq::TIMER_IRQ_NUM, true);
    }
}
