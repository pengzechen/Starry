use core::arch::global_asm;

// use aarch64_cpu::registers::{ESR_EL1, FAR_EL1};
// use tock_registers::interfaces::Readable;

// use hypercraft::lower_aarch64_synchronous;

use super::TrapFrame;

global_asm!(include_str!("trap_el2.S"));

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

#[no_mangle]
fn invalid_exception_el2(tf: &mut TrapFrame, kind: TrapKind, source: TrapSource) {
    panic!(
        "Invalid exception {:?} from {:?}:\n{:#x?}",
        kind, source, tf
    );
}

#[no_mangle]
fn handle_irq_exception_el2(_tf: &mut TrapFrame) {
    crate::trap::handle_irq_extern(0)
}
