// Copyright (c) 2023 Beihang University, Huawei Technologies Co.,Ltd. All rights reserved.
// Rust-Shyper is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use hypercraft::arch::{ContextFrame, ContextFrameTrait};
use hypercraft::arch::vcpu::VmCpuRegisters;
use hypercraft::arch::hvc::{HVC_SYS, HVC_SYS_BOOT, hvc_guest_handler};
use hypercraft::arch::emu::EmuContext;

use axhal::arch::hv::exception_utils::*;
// use axhal::{gic_is_priv, gic_lrs, GICD, GICH, GICV, GICC};

use super::guest_psci::smc_guest_handler;
use super::current_cpu;
use super::emu::emu_handler;
use super::interrupt::handle_virtual_interrupt;

const HVC_RETURN_REG: usize = 0;
const SMC_RETURN_REG: usize = 0;

#[no_mangle]
pub extern "C" fn data_abort_handler(ctx: &mut ContextFrame) {
    current_cpu().set_ctx(ctx);

    let emu_ctx = EmuContext {
        address: exception_fault_addr(),
        width: exception_data_abort_access_width(),
        write: exception_data_abort_access_is_write(),
        sign_ext: exception_data_abort_access_is_sign_ext(),
        reg: exception_data_abort_access_reg(),
        reg_width: exception_data_abort_access_reg_width(),
    };
    // if ctx.exception_pc() == 0xffffa23d3a94fc6c {
        // read_timer_regs();
    // }
    debug!("[data_abort_handler] data fault addr {:#x}, esr: {:#x}, elr:{:#x} is_write:{}",
        exception_fault_addr(), exception_esr(), ctx.exception_pc(), emu_ctx.write);
    let elr = ctx.exception_pc();

    if !exception_data_abort_handleable() {
        panic!(
            "Data abort not handleable 0x{:#x}, esr 0x{:#x}",
            exception_fault_addr(),
            exception_esr()
        );
    }

    if !exception_data_abort_is_translate_fault() {
        // No migrate need
        panic!(
            "Data abort is not translate fault 0x{:x}\n ctx: {}",
            exception_fault_addr(), ctx
        );           
    }

    if !emu_handler(&emu_ctx) {
        // active_vm().unwrap().show_pagetable(emu_ctx.address);
        info!(
            "[data_abort_handler] write {}, width {}, reg width {}, addr {:x}, iss {:x}, reg idx {}, reg val 0x{:x}, esr 0x{:x}",
            exception_data_abort_access_is_write(),
            emu_ctx.width,
            emu_ctx.reg_width,
            emu_ctx.address,
            exception_iss(),
            emu_ctx.reg,
            ctx.gpr(emu_ctx.reg),
            exception_esr()
        );
        panic!(
            "data_abort_handler: Failed to handler emul device request, ipa 0x{:x} elr 0x{:x}",
            emu_ctx.address, elr
        );
    }

    let val = elr + exception_next_instruction_step();
    ctx.set_exception_pc(val);

    current_cpu().clear_ctx();
}

#[no_mangle]
pub extern "C" fn hvc_handler(ctx: &mut ContextFrame) {
    let x0 = ctx.gpr(0);
    let x1 = ctx.gpr(1);
    let x2 = ctx.gpr(2);
    let x3 = ctx.gpr(3);
    let x4 = ctx.gpr(4);
    let x5 = ctx.gpr(5);
    let x6 = ctx.gpr(6);
    let mode = ctx.gpr(7);
    debug!("[hvc_handler]: mode:{}", mode);
    let hvc_type = (mode >> 8) & 0xff;
    let event = mode & 0xff;

    current_cpu().set_ctx(ctx);
/* 
    let misr = GICH.get_misr();
    let hcr = GICH.get_hcr();
    let gicv_ctlr = GICV.get_ctlr();
    let eisr0 = GICH.get_eisr_by_idx(0);
    let lr0 = GICH.get_lr_by_idx(0);
    let gicc_ctl = GICC.get_ctlr();
    debug!("[hvc_handler] why!!!!!!!!!!!!!!! misr: {:#x} eisr0:{:#x} lr0:{:#x} hcr:{:#x} gicv_ctlr:{:#x} gicc_ctl:{:#x}", misr, eisr0,lr0, hcr, gicv_ctlr, gicc_ctl);

    debug!("this is x0: {}", x0);
    let prio25 = GICD.lock().get_priority(25);
    let prio27 = GICD.lock().get_priority(27);
    let state = GICD.lock().get_enable(25);
    debug!("[hvc_handler] 25 enabled:{} prio25 {:#x} prio27 {:#x}", state, prio25, prio27);
    //axhal::gicc_clear_current_irq(x0, false);
    // axhal::gicv_clear_current_irq(x0, false);
*/    
    ctx.set_gpr(HVC_RETURN_REG, 0);

    // handle_virtual_interrupt(79, 0);
/*  let misr = GICH.get_misr();
    let hcr = GICH.get_hcr();
    let gicv_eoi = GICV.get_ctlr();
    let gicv_iar = GICV.get_iar();
    let eisr0 = GICH.get_eisr_by_idx(0);
    let lr0 = GICH.get_lr_by_idx(0);
    let gicc_iar = GICC.get_iar();
    debug!("after inject misr: {:#x} eisr0:{:#x} lr0:{:#x} hcr:{:#x} gicv_ctlr:{:#x} gicv_iar:{:#x} gicc_iar:{:#x}", misr, eisr0,lr0, hcr, gicv_eoi, gicv_iar, gicc_iar);
*/
 /*
    match hvc_guest_handler(hvc_type, event, x0, x1, x2, x3, x4, x5, x6) {
        Ok(val) => {
            ctx.set_gpr(HVC_RETURN_REG, val);
        }
        Err(_) => {
            warn!("Failed to handle hvc request fid 0x{:x} event 0x{:x}", hvc_type, event);
            ctx.set_gpr(HVC_RETURN_REG, usize::MAX);
        }
    }
   
    if hvc_type==HVC_SYS && event== HVC_SYS_BOOT {
        unsafe {
            let regs: &mut VmCpuRegisters = core::mem::transmute(x1);   // x1 is the vm regs context
            // save arceos context
            regs.save_for_os_context_regs.gpr = ctx.gpr;
            regs.save_for_os_context_regs.sp = ctx.sp;
            regs.save_for_os_context_regs.elr = ctx.elr;
            regs.save_for_os_context_regs.spsr = ctx.spsr;

            ctx.gpr = regs.guest_trap_context_regs.gpr;
            ctx.sp = regs.guest_trap_context_regs.sp;
            ctx.elr = regs.guest_trap_context_regs.elr;
            ctx.spsr = regs.guest_trap_context_regs.spsr;
        }
    }
     */
    current_cpu().clear_ctx();
}

#[no_mangle]
pub extern "C" fn smc_handler(ctx: &mut ContextFrame) {
    let fid = ctx.gpr(0);
    let x1 = ctx.gpr(1);
    let x2 = ctx.gpr(2);
    let x3 = ctx.gpr(3);

    current_cpu().set_ctx(ctx);

    match smc_guest_handler(fid, x1, x2, x3) {
        Ok(val) => {
            ctx.set_gpr(SMC_RETURN_REG, val);
        }
        Err(_) => {
            warn!("Failed to handle smc request fid 0x{:x}", fid);
            ctx.set_gpr(SMC_RETURN_REG, usize::MAX);
        }
    }

    let elr = ctx.exception_pc();
    let val = elr + exception_next_instruction_step();
    ctx.set_exception_pc(val);

    current_cpu().clear_ctx();
}
