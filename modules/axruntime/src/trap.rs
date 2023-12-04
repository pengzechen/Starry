#[cfg(all(feature = "hv", target_arch = "aarch64"))]
use crate::hv::aarch64_kernel::handle_virtual_interrupt;
struct TrapHandlerImpl;

#[crate_interface::impl_interface]
impl axhal::trap::TrapHandler for TrapHandlerImpl {
    fn handle_irq(_irq_num: usize) {
        #[cfg(feature = "irq")]
        {
            let guard = kernel_guard::NoPreempt::new();
            axhal::irq::dispatch_irq(_irq_num);
            drop(guard); // rescheduling may occur when preemption is re-enabled.
        }
    }
    #[cfg(all(feature = "hv", target_arch = "aarch64"))]
    fn handle_irq_hv(irq_num: usize, src: usize) {
        // if axhal::irq::irq_num_exist(irq_num) {
            let guard = kernel_guard::NoPreempt::new();
            axhal::irq::dispatch_irq(irq_num);
            drop(guard);
        // }else {  // sgi
        //    handle_virtual_interrupt(irq_num, src);
        // }
    }
}
