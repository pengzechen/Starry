use axconfig::{SMP, TASK_STACK_SIZE};
use axhal::mem::{virt_to_phys, VirtAddr};
use core::sync::atomic::{AtomicUsize, Ordering};

use aarch64_cpu::{asm, asm::barrier, registers::*};
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

#[link_section = ".bss.stack"]
static mut SECONDARY_BOOT_STACK: [[u8; TASK_STACK_SIZE]; SMP - 1] = [[0; TASK_STACK_SIZE]; SMP - 1];

static ENTERED_CPUS: AtomicUsize = AtomicUsize::new(1);

pub fn start_secondary_cpus(primary_cpu_id: usize) {
    let mut logic_cpu_id = 0;
    for i in 0..SMP {
        if i != primary_cpu_id {
            let stack_top = virt_to_phys(VirtAddr::from(unsafe {
                SECONDARY_BOOT_STACK[logic_cpu_id].as_ptr_range().end as usize
            }));

            debug!("starting CPU {}...", i);
            axhal::mp::start_secondary_cpu(i, stack_top);
            logic_cpu_id += 1;

            while ENTERED_CPUS.load(Ordering::Acquire) <= logic_cpu_id {
                core::hint::spin_loop();
            }
        }
    }
}

/// The main entry point of the ArceOS runtime for secondary CPUs.
///
/// It is called from the bootstrapping code in [axhal].
#[no_mangle]
pub extern "C" fn rust_main_secondary(cpu_id: usize) -> ! {
    ENTERED_CPUS.fetch_add(1, Ordering::Relaxed);
    info!("Secondary CPU {} started.", cpu_id);

    #[cfg(all(feature = "hv", target_arch = "riscv64"))]
    hypercraft::init_hv_runtime();

    #[cfg(not(feature = "hv"))]
    {
        #[cfg(feature = "paging")]
        {
            super::remap_kernel_memory().unwrap();
        }
    }

    axhal::platform_init_secondary();

    #[cfg(feature = "multitask")]
    axtask::init_scheduler_secondary();

    info!("Secondary CPU {} init OK.", cpu_id);
    super::INITED_CPUS.fetch_add(1, Ordering::Relaxed);

    while !super::is_init_ok() {
        core::hint::spin_loop();
    }

    #[cfg(feature = "irq")]
    axhal::arch::enable_irqs();

    // run multi vm, this will not return
    // todo: add more feature for multi vm
    #[cfg(feature = "hv")]
    
    unsafe {
        secondary_vm(cpu_id);
    }
    info!("Secondary CPU {} ---=---", cpu_id);
    #[cfg(feature = "multitask")]
    {
        debug!("secondary CPU {} enter idle loop", cpu_id);
        axtask::run_idle();
    }

    #[cfg(not(feature = "multitask"))]
    loop {
        axhal::arch::wait_for_irqs();
        #[cfg(feature = "hv")]
        {
            debug!("after wfi!!!!!!!!!!!");
            // crate::hv::secondary_main_hv(cpu_id);
        }
    }
}

extern "C" {
    fn secondary_vm(cpu_id: usize) -> !;
}