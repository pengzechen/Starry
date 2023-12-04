use hypercraft::VCpu;
use axhal::arch::hv::ipi::*;
use super::current_cpu;
use super::vm_array::{init_vm_vcpu, run_vm_vcpu};
use crate::hv::HyperCraftHalImpl;

pub(crate) fn psci_ipi_handler(msg: &IpiMessage) {
    match &msg.ipi_message {
        IpiInnerMsg::Power(power_msg) => {
            // only one vcpu for a pcpu and only one vm. need to modify in the future
            let trgt_vcpu = current_cpu().get_active_vcpu();
            match power_msg.event {
                PowerEvent::PsciIpiCpuOn => {
                    /*
                    if trgt_vcpu.state() != VcpuState::Inv {
                        warn!(
                            "psci_ipi_handler: target VCPU {} in VM {} is already running",
                            trgt_vcpu.id(),
                            trgt_vcpu.vm().unwrap().id()
                        );
                        return;
                    }
                    */
                    info!(
                        "Core {} (vm {}, vcpu {}) is woke up",
                        current_cpu().cpu_id,
                        trgt_vcpu.vm_id,
                        trgt_vcpu.vcpu_id
                    );
                    psci_vcpu_on(trgt_vcpu, power_msg.entry, power_msg.context);
                }
                PowerEvent::PsciIpiCpuOff => {
                    warn!("PowerEvent::PsciIpiCpuOff")
                }
            }
        }
        _ => {
            error!("psci_ipi_handler: receive illegal psci ipi type");
        }
    }
}

fn psci_vcpu_on(vcpu: &mut VCpu<HyperCraftHalImpl>, entry: usize, ctx: usize) {
    debug!("psci vcpu on， entry {:x}, ctx {:x} currentcpu:{}", entry, ctx, current_cpu().cpu_id);
    init_vm_vcpu(vcpu.vm_id, vcpu.vcpu_id, entry, ctx);
    run_vm_vcpu(vcpu.vm_id, vcpu.vcpu_id);
    
    /* if vcpu.phys_id() != current_cpu().id {
        panic!(
            "cannot psci on vcpu on cpu {} by cpu {}",
            vcpu.phys_id(),
            current_cpu().cpu_id
        );
    }
    current_cpu().cpu_state = CpuState::CpuRun;
    vcpu.reset_context();
    vcpu.set_gpr(0, ctx);
    vcpu.set_elr(entry);
    // Just wake up the vcpu and
    // invoke current_cpu().sched.schedule()
    // let the scheduler enable or disable timer
    current_cpu().scheduler().wakeup(vcpu);
    current_cpu().scheduler().do_schedule();
    */
}