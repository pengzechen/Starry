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
// use hypercraft::arch::ContextFrame;
// use hypercraft::arch::ContextFrameTrait;
use tock_registers::interfaces::*;

// use super::exception_utils::*;
// use crate::platform::aarch64_common::gic::*;

global_asm!(include_str!("exception.S"));
use super::TrapFrame;

// extern "C" {
//     fn data_abort_handler(ctx: &mut ContextFrame);
//     fn hvc_handler(ctx: &mut ContextFrame);
//     fn smc_handler(ctx: &mut ContextFrame);
// }

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

/// deal with invalid aarch64 synchronous exception
#[no_mangle]
fn invalid_exception_el2(tf: &mut TrapFrame, kind: TrapKind, source: TrapSource) {
    panic!(
        "Invalid exception {:?} from {:?}:\n{:#x?}",
        kind, source, tf
    );
}

// /// deal with lower aarch64 interruption exception
// #[no_mangle]
// fn current_spxel_irq(ctx: &mut ContextFrame) {
//     debug!("IRQ stay in the same el!!!!!!!!!!!!!!!");
//     lower_aarch64_irq(ctx);
// }

// /// deal with lower aarch64 interruption exception
// #[no_mangle]
// fn lower_aarch64_irq(ctx: &mut ContextFrame) {
//     debug!("IRQ routed to EL2!!!!!!!!!!!!!!!");
//     // read_timer_regs();
//     let (irq, src) = gicc_get_current_irq();
//     debug!("src {} id{}", src, irq);
//     crate::trap::handle_irq_extern_hv(irq, src, ctx);
// }

// /// deal with lower aarch64 synchronous exception
// #[no_mangle]
// fn lower_aarch64_synchronous(ctx: &mut ContextFrame) {
//     debug!(
//         "enter lower_aarch64_synchronous exception class:0x{:X}",
//         exception_class()
//     );
//     // current_cpu().set_context_addr(ctx);

//     match exception_class() {
//         0x24 => {
//             // info!("Core[{}] data_abort_handler", cpu_id());
//             unsafe {
//                 data_abort_handler(ctx);
//             }
//         }
//         0x16 => unsafe {
//             hvc_handler(ctx);
//         },
//         0x17 => unsafe {
//             smc_handler(ctx);
//         },
//         // 0x18 todo？
//         _ => {
//             panic!(
//                 "handler not presents for EC_{} @ipa 0x{:x}, @pc 0x{:x}, @esr 0x{:x}, @sctlr_el1 0x{:x}, @vttbr_el2 0x{:x}, @vtcr_el2: {:#x} hcr: {:#x} ctx:{}",
//                 exception_class(),
//                 exception_fault_addr(),
//                 (*ctx).exception_pc(),
//                 exception_esr(),
//                 cortex_a::registers::SCTLR_EL1.get() as usize,
//                 cortex_a::registers::VTTBR_EL2.get() as usize,
//                 cortex_a::registers::VTCR_EL2.get() as usize,
//                 cortex_a::registers::HCR_EL2.get() as usize,
//                 ctx
//             );
//         }
//     }
// }
