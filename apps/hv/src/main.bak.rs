#![no_std]
#![no_main]
extern crate alloc;
#[macro_use] extern crate axstd;
use log::*;

use dtb_aarch64::MachineMeta;
use aarch64_config::*;
use axstd::info;
use axstd::hv::{
        GuestPageTable, GuestPageTableTrait, HyperCraftHalImpl, PerCpu,
        Result, VM, VcpusArray, 
        VM_ARRAY, VM_MAX_NUM,
        add_vm, add_vm_vcpu, 
        init_vm_vcpu, init_vm_emu_device, init_vm_passthrough_device, 
        is_vcpu_primary_ok,
        run_vm_vcpu, 
};
mod dtb_aarch64;
mod aarch64_config;
use alloc::vec::Vec;
use page_table_entry::MappingFlags;
#[cfg(feature = "rk3588")]
pub mod rk3588;

#[cfg(feature = "qemu")]
pub mod qemu;

#[no_mangle] fn main(hart_id:usize) {
    #[cfg(feature = "rk3588")]
    rk3588::main(hart_id);

    #[cfg(feature = "qemu")]
    qemu::main(hart_id);
}
