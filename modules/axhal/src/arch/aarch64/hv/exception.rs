// Copyright (c) 2023 Beihang University, Huawei Technologies Co.,Ltd. All rights reserved.
// Rust-Shyper is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use core::arch::global_asm;
use tock_registers::interfaces::*;

use hypercraft::arch::ContextFrame;
use hypercraft::arch::ContextFrameTrait;

use crate::platform::aarch64_common::gic::*;
use super::sync::{data_abort_handler, hvc_handler, smc_handler};

global_asm!(include_str!("exception.S"));

#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
enum TrapKind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
}

#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
enum TrapSource {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

#[inline(always)]
pub fn exception_esr() -> usize {
    cortex_a::registers::ESR_EL2.get() as usize
}

#[inline(always)]
pub fn exception_esr_el1() -> usize {
    cortex_a::registers::ESR_EL1.get() as usize
}

#[inline(always)]
fn exception_class() -> usize {
    (exception_esr() >> 26) & 0b111111
}

#[inline(always)]
fn exception_far() -> usize {
    cortex_a::registers::FAR_EL2.get() as usize
}

#[inline(always)]
fn exception_hpfar() -> usize {
    let hpfar: u64;
    mrs!(hpfar, HPFAR_EL2);
    hpfar as usize
}

#[allow(non_upper_case_globals)]
const ESR_ELx_S1PTW_SHIFT: usize = 7;
#[allow(non_upper_case_globals)]
const ESR_ELx_S1PTW: usize = 1 << ESR_ELx_S1PTW_SHIFT;

macro_rules! arm_at {
    ($at_op:expr, $addr:expr) => {
        unsafe {
            core::arch::asm!(concat!("AT ", $at_op, ", {0}"), in(reg) $addr, options(nomem, nostack));
            core::arch::asm!("isb");
        }
    };
}

fn translate_far_to_hpfar(far: usize) -> Result<usize, ()> {
    /*
     * We have
     *	PAR[PA_Shift - 1 : 12] = PA[PA_Shift - 1 : 12]
     *	HPFAR[PA_Shift - 9 : 4]  = FIPA[PA_Shift - 1 : 12]
     */
    // #define PAR_TO_HPFAR(par) (((par) & GENMASK_ULL(PHYS_MASK_SHIFT - 1, 12)) >> 8)
    fn par_to_far(par: u64) -> u64 {
        let mask = ((1 << (52 - 12)) - 1) << 12;
        (par & mask) >> 8
    }

    use cortex_a::registers::PAR_EL1;

    let par = PAR_EL1.get();
    arm_at!("s1e1r", far);
    let tmp = PAR_EL1.get();
    PAR_EL1.set(par);
    if (tmp & PAR_EL1::F::TranslationAborted.value) != 0 {
        Err(())
    } else {
        Ok(par_to_far(tmp) as usize)
    }
}

// addr be ipa
#[inline(always)]
pub fn exception_fault_addr() -> usize {
    let far = exception_far();
    let hpfar = if (exception_esr() & ESR_ELx_S1PTW) == 0 && exception_data_abort_is_permission_fault() {
        translate_far_to_hpfar(far).unwrap_or_else(|_| {
            info!("error happen in translate_far_to_hpfar");
            0
        })
    } else {
        exception_hpfar()
    };
    (far & 0xfff) | (hpfar << 8)
}

/// \return 1 means 32-bit instruction, 0 means 16-bit instruction
#[inline(always)]
fn exception_instruction_length() -> usize {
    (exception_esr() >> 25) & 1
}

#[inline(always)]
pub fn exception_next_instruction_step() -> usize {
    2 + 2 * exception_instruction_length()
}

#[inline(always)]
pub fn exception_iss() -> usize {
    exception_esr() & ((1 << 25) - 1)
}

#[inline(always)]
pub fn exception_data_abort_handleable() -> bool {
    (!(exception_iss() & (1 << 10)) | (exception_iss() & (1 << 24))) != 0
}

#[inline(always)]
pub fn exception_data_abort_is_translate_fault() -> bool {
    (exception_iss() & 0b111111 & (0xf << 2)) == 4
}

#[inline(always)]
pub fn exception_data_abort_is_permission_fault() -> bool {
    (exception_iss() & 0b111111 & (0xf << 2)) == 12
}

#[inline(always)]
pub fn exception_data_abort_access_width() -> usize {
    1 << ((exception_iss() >> 22) & 0b11)
}

#[inline(always)]
pub fn exception_data_abort_access_is_write() -> bool {
    (exception_iss() & (1 << 6)) != 0
}

#[inline(always)]
pub fn exception_data_abort_access_in_stage2() -> bool {
    (exception_iss() & (1 << 7)) != 0
}

#[inline(always)]
pub fn exception_data_abort_access_reg() -> usize {
    (exception_iss() >> 16) & 0b11111
}

#[inline(always)]
pub fn exception_data_abort_access_reg_width() -> usize {
    4 + 4 * ((exception_iss() >> 15) & 1)
}

#[inline(always)]
pub fn exception_data_abort_access_is_sign_ext() -> bool {
    ((exception_iss() >> 21) & 1) != 0
}

/// deal with invalid aarch64 synchronous exception
#[no_mangle]
fn invalid_exception_el2(tf: &mut ContextFrame, kind: TrapKind, source: TrapSource) {
    panic!(
        "Invalid exception {:?} from {:?}:\n{:#x?}",
        kind, source, tf
    );
}

/// deal with lower aarch64 interruption exception
#[no_mangle]
fn current_spxel_irq(ctx: &mut ContextFrame) {
    debug!("[current_spxel_irq] ");
    lower_aarch64_irq(ctx);
}

/// deal with lower aarch64 interruption exception
#[no_mangle]
fn lower_aarch64_irq(ctx: &mut ContextFrame) {
    debug!("IRQ routed to EL2");
    let (irq, src) = gicc_get_current_irq();
    debug!("src {} id{}", src, irq);
    crate::trap::handle_irq_extern_hv(irq, src);
    // deactivate_irq(irq);
    /* 
    if let Some(irq_id) = pending_irq() {
        // deactivate_irq(irq_id);
        inject_irq(irq_id);
    }
    */
}

/// deal with lower aarch64 synchronous exception
#[no_mangle]
fn lower_aarch64_synchronous(ctx: &mut ContextFrame) {
    debug!("enter lower_aarch64_synchronous exception class:0x{:X}", exception_class());
    // current_cpu().set_context_addr(ctx);

    match exception_class() {
        0x24 => {
            // info!("Core[{}] data_abort_handler", cpu_id());
            data_abort_handler(ctx);
        }
        0x16 => {
            hvc_handler(ctx);
        }
        0x17 => {
            smc_handler(ctx);
        }
        // 0x18 todo？
        _ => {   
            panic!(
                "handler not presents for EC_{} @ipa 0x{:x}, @pc 0x{:x}, @esr 0x{:x}, @sctlr_el1 0x{:x}, @vttbr_el2 0x{:x}, @vtcr_el2: {:#x} hcr: {:#x} ctx:{}",
                exception_class(),
                exception_fault_addr(),
                (*ctx).exception_pc(),
                exception_esr(),
                cortex_a::registers::SCTLR_EL1.get() as usize,
                cortex_a::registers::VTTBR_EL2.get() as usize,
                cortex_a::registers::VTCR_EL2.get() as usize,
                cortex_a::registers::HCR_EL2.get() as usize,
                ctx
            );
        },
    }
}
