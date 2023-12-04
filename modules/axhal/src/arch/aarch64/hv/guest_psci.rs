use smccc::psci::*;

use super::ipi::*;

const PSCI_RET_SUCCESS: usize = 0;
const PSCI_RET_NOT_SUPPORTED: usize = 0xffff_ffff_ffff_ffff;   //-1
const PSCI_RET_INVALID_PARAMS: usize = 0xffff_ffff_ffff_fffe;   // -2
const PSCI_RET_ALREADY_ON: usize = 0xffff_ffff_ffff_fffc;   // -4

const PSCI_TOS_NOT_PRESENT_MP: usize = 2;

#[inline(never)]
pub fn smc_guest_handler(
    fid: usize, 
    x1: usize, 
    x2: usize, 
    x3: usize,
) -> Result<usize, ()>  {
    debug!(
        "smc_guest_handler: fid {:#x}, x1 {:#x}, x2 {:#x}, x3 {:#x}",
        fid, x1, x2, x3
    );
    let r = match fid as u32 {
        PSCI_FEATURES => match x1 as u32 {
            PSCI_VERSION | PSCI_CPU_ON_64 | PSCI_FEATURES => Ok(PSCI_RET_SUCCESS),
            // | PSCI_CPU_SUSPEND_64| PSCI_SYSTEM_SUSPEND_64
            // | PSCI_SYSTEM_RESET2_64 => Ok(PSCI_RET_SUCCESS),
            _ => Ok(PSCI_RET_NOT_SUPPORTED),
        },
        PSCI_VERSION => Ok(smc_call(PSCI_VERSION, 0, 0, 0).0),
        PSCI_CPU_ON_64 => Ok(psci_guest_cpu_on(x1, x2, x3)),
       
        /*PSCI_CPU_ON_64 => {
            unsafe {
                run_vm_vcpu(0, 1);
            }
            
            let smc_ret = smc_call(PSCI_CPU_ON_64, x1, x2, x3).0;
            if smc_ret == 0 {
                Ok(0)
            }else {
                // todo();
                Ok(0)
            }
        },*/
        // PSCI_SYSTEM_RESET => psci_guest_sys_reset(),
        PSCI_SYSTEM_RESET => Ok(smc_call(PSCI_SYSTEM_RESET, 0, 0, 0).0),
        // PSCI_SYSTEM_OFF => psci_guest_sys_off(),
        PSCI_SYSTEM_OFF => Ok(smc_call(PSCI_SYSTEM_OFF, 0, 0, 0).0),
        PSCI_MIGRATE_INFO_TYPE => Ok(PSCI_TOS_NOT_PRESENT_MP),
        PSCI_AFFINITY_INFO_64 => Ok(0),
        _ => Err(()),
    };
    debug!(
        "smc_guest_handler: fid {:#x}, x1 {:#x}, x2 {:#x}, x3 {:#x} result: {:#x}",
        fid, x1, x2, x3, r.unwrap(),
    );
    r
}

fn psci_guest_cpu_on(mpidr: usize, entry: usize, ctx: usize) -> usize {
    debug!("this is vcpu id {}, entry:{:#x} ctx:{:#x}", mpidr, entry, ctx);
    let pcpu_id = mpidr & 0xff; // vcpu and pcpu id are the same
    let m = IpiPowerMessage {
        src: 0,     //vm id
        event: PowerEvent::PsciIpiCpuOn,
        entry,
        context: ctx,
    };

    if !ipi_send_msg(pcpu_id, IpiType::Power, IpiInnerMsg::Power(m)) {
        warn!("psci_guest_cpu_on: fail to send msg");
        return usize::MAX - 1;
    }
    0
}

pub fn psci_ipi_handler(msg: IpiMessage) {
    /* 
    match msg.ipi_message {
        IpiInnerMsg::Power(power_msg) => {
            let trgt_vcpu = match current_cpu().vcpu_array.pop_vcpu_through_vmid(power_msg.src) {
                None => {
                    warn!(
                        "Core {} failed to find target vcpu, source vmid {}",
                        current_cpu().id,
                        power_msg.src
                    );
                    return;
                }
                Some(vcpu) => vcpu,
            };
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
                    info!(
                        "Core {} (vm {}, vcpu {}) is woke up",
                        current_cpu().id,
                        trgt_vcpu.vm().unwrap().id(),
                        trgt_vcpu.id()
                    );*/
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
    }*/
}

#[inline(never)]
pub fn smc_call(x0: u32, x1: usize, x2: usize, x3: usize) -> (usize, usize, usize, usize) {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let r0;
        let r1;
        let r2;
        let r3;
        core::arch::asm!(
            "smc #0",
            inout("x0") x0 as usize => r0,
            inout("x1") x1 => r1,
            inout("x2") x2 => r2,
            inout("x3") x3 => r3,
            options(nomem, nostack)
        );
        (r0, r1, r2, r3)
    }

    #[cfg(not(target_arch = "aarch64"))]
    error!("smc not supported");
}