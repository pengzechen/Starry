struct TrapHandlerImpl;


use hypercraft::arch::ContextFrame;
#[cfg(all(feature = "hv"))]
use crate::hv::kernel::{handle_virtual_interrupt, current_cpu};

#[crate_interface::impl_interface]
impl axhal::trap::TrapHandler for TrapHandlerImpl {
    fn handle_irq(_irq_num: usize, _from_user: bool) {
        #[cfg(feature = "irq")]
        {
            let guard = kernel_guard::NoPreempt::new();
            axhal::irq::dispatch_irq(_irq_num);
            drop(guard); // rescheduling may occur when preemption is re-enabled.
        }
    }

    #[cfg(feature = "signal")]
    fn handle_signal() {
        unimplemented();
    }
    
    #[cfg(all(feature = "hv"))]
    fn handle_irq_hv(irq_num: usize, src: usize, ctx: &mut ContextFrame) {
        
        current_cpu().set_ctx(ctx);
        if axhal::irq::irq_num_exist(irq_num) {
            let guard = kernel_guard::NoPreempt::new();
            axhal::irq::dispatch_irq(irq_num);
            drop(guard);
        }else {
            handle_virtual_interrupt(irq_num, src);
        }
        
        //debug!("[handle_irq_hv] before deactivate irq {} ", irq_num);
        
        if irq_num==axhal::IPI_IRQ_NUM || irq_num==axhal::MAINTENANCE_IRQ_NUM || irq_num==axhal::time::HYPERVISOR_TIMER_IRQ_NUM {
            axhal::gicc_clear_current_irq(irq_num, true);
        }  else {
            axhal::gicc_clear_current_irq(irq_num, false);
        }

        //debug!("[handle_irq_hv] after deactivate irq {} ", irq_num);
        current_cpu().clear_ctx();
        
    }
}
