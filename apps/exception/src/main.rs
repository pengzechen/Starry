#![no_std]
#![no_main]

use core::arch::asm;
use libax::println;

fn raise_break_exception() {
    unsafe {
        asm!("brk #0");
    }
}

#[no_mangle]
fn main() {
    println!("Running exception tests...");
    raise_break_exception();
    println!("Exception tests run OK!");
}
