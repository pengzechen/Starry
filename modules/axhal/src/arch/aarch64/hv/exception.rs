// Copyright (c) 2023 Beihang University, Huawei Technologies Co.,Ltd. All rights reserved.
// Rust-Shyper is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use super::exception_utils::*;
use alloc::collections::BTreeMap;
use core::arch::global_asm;
use spin::RwLock;
use tock_registers::interfaces::*;

global_asm!(include_str!("exception.S"));
use super::TrapFrame;

type ExceptionHandler = fn(&mut TrapFrame);
static LOWER_AARCH64_SYNCHRONOUS_HANDLERS: RwLock<BTreeMap<usize, ExceptionHandler>> =
    RwLock::new(BTreeMap::new());

// esr fields [31:26]
const MAX_EXCEPTION_COUNT: usize = 64;

/// AARCH64 Exception handler registration.
///
/// It also enables the IRQ if the registration succeeds. It returns `false` if
/// the registration failed.
#[allow(dead_code)]
pub fn register_exception_handler_aarch64(exception_class: usize, handler: ExceptionHandler) -> bool {
    if exception_class < MAX_EXCEPTION_COUNT {
        let mut handlers = LOWER_AARCH64_SYNCHRONOUS_HANDLERS.write();
        handlers.insert(exception_class, handler);
        return true;
    }
}

fn dispatch_exception(exception_class: usize, tf: &mut TrapFrame) {
    let handlers = LOWER_AARCH64_SYNCHRONOUS_HANDLERS.read();
    if let Some(handler) = handlers.get(&exception_class) {
        handler(tf);
    } else {
        panic!(
            "handler not presents for EC_{} @ipa 0x{:x}, @pc 0x{:x}, @esr 0x{:x}, @sctlr_el1 0x{:x}, @vttbr_el2 0x{:x}, @vtcr_el2: {:#x} hcr: {:#x} ctx:{}",
            exception_class(),
            exception_fault_addr(),
            tf.elr,
            exception_esr(),
            cortex_a::registers::SCTLR_EL1.get() as usize,
            cortex_a::registers::VTTBR_EL2.get() as usize,
            cortex_a::registers::VTCR_EL2.get() as usize,
            cortex_a::registers::HCR_EL2.get() as usize,
            tf
        );
    }
}

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
// fn current_spxel_irq(ctx: &mut TrapFrame) {
//     lower_aarch64_irq(ctx);
// }

// /// deal with lower aarch64 interruption exception
// #[no_mangle]
// fn lower_aarch64_irq(ctx: &mut TrapFrame) {
//     let (irq, src) = gicc_get_current_irq();
//     debug!("src {} id{}", src, irq);
//     crate::trap::handle_irq_extern_hv(irq, src, ctx);
// }

/// deal with lower aarch64 synchronous exception
#[no_mangle]
fn lower_aarch64_synchronous(tf: &mut TrapFrame) {
    debug!(
        "enter lower_aarch64_synchronous exception class:0x{:X}",
        exception_class()
    );
    // 0x16: hvc_handler
    // 0x24: data_abort_handler
    dispatch_exception(exception_class(), tf);
}
