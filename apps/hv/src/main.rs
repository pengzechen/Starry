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

/*
 * 运行需要nimbos启用gicv3
 * 
*/

#[no_mangle] fn main(hart_id: usize) {
    println!("Hello, hv!");

    {
        // qemu-virt
        let vm1_kernel_entry = 0x7020_0000;
        let vm1_dtb = 0x7000_0000;

        // boot cpu
        PerCpu::<HyperCraftHalImpl>::init(0).unwrap(); 
        // get current percpu
        let percpu = PerCpu::<HyperCraftHalImpl>::ptr_for_cpu(hart_id);
        // create vcpu, need to change addr for aarch64!
        let gpt = setup_gpm(vm1_dtb, vm1_kernel_entry).unwrap();  
        let vcpu = percpu.create_vcpu(0, 0).unwrap();
        percpu.set_active_vcpu(Some(vcpu.clone()));

        let vcpus = VcpusArray::new();

        // add vcpu into vm
        let vm: VM<HyperCraftHalImpl, GuestPageTable> = VM::new(vcpus, gpt, 0).unwrap();
        unsafe {
            let mut vm_array = Vec::with_capacity(VM_MAX_NUM);
            for _ in 0..VM_MAX_NUM {
                vm_array.push(None);
            }
            VM_ARRAY.init_by(vm_array);
            debug!("this is VM_ARRAY: {:p}", &VM_ARRAY as *const _);
        }

        add_vm(0, vm);
        let vcpu_id = vcpu.vcpu_id;
        add_vm_vcpu(0, vcpu);
        init_vm_vcpu(0, vcpu_id, vm1_kernel_entry, vm1_dtb);
        init_vm_emu_device(0);
        init_vm_passthrough_device(0);

        run_vm_vcpu(0, 0);
    }
}

#[no_mangle] pub extern "C" fn secondary_vm(cpu_id: usize)  {
    while !is_vcpu_primary_ok() {
        core::hint::spin_loop();
    }
    // let vm2_kernel_entry = 0x5020_0000;
    // let vm2_dtb = 0x5000_0000;
    
    // PerCpu::<HyperCraftHalImpl>::setup_this_cpu(cpu_id).unwrap();
    // let percpu = PerCpu::<HyperCraftHalImpl>::this_cpu();
    // let virt_cpu = percpu.create_vcpu(1, 0).unwrap();
    // percpu.set_active_vcpu(Some(virt_cpu.clone()));
    // let vcpus = VcpusArray::new();

    // let gpt = setup_gpm(vm2_dtb, vm2_kernel_entry).unwrap(); 
    // let vm: VM<HyperCraftHalImpl, GuestPageTable> = VM::new(vcpus, gpt, 1).unwrap();

    // add_vm(1, vm);
    // let vcpu_id = virt_cpu.vcpu_id;
    // add_vm_vcpu(1, virt_cpu);
    // init_vm_vcpu(1, vcpu_id, vm2_kernel_entry, vm2_dtb);
    // init_vm_emu_device(1);
    // init_vm_passthrough_device(1);

    // run_vm_vcpu(1, 0);
}

pub fn setup_gpm(dtb: usize, kernel_entry: usize) -> Result<GuestPageTable> {
    let mut gpt = GuestPageTable::new()?;
    let meta = MachineMeta::parse(dtb);

    // hard code for virtio_mmio
    gpt.map_region(
        0xa000000,
        0xa000000,
        0x4000,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
    )?;
    debug!("map virtio");   // ok
    
    if kernel_entry == 0x7020_0000 {
        if let Some(pl011) = meta.pl011 {
            gpt.map_region(
                pl011.base_address,
                pl011.base_address,
                pl011.size,
                MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
            )?;
        }
        debug!("map pl011");
    }
    
    if let Some(pl031) = meta.pl031 {
        gpt.map_region(
            pl031.base_address,
            pl031.base_address,
            pl031.size,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
    }
    debug!("map pl031");
    if let Some(pl061) = meta.pl061 {
        gpt.map_region(
            pl061.base_address,
            pl061.base_address,
            pl061.size,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
    }
    debug!("map pl061");

    // gicv3 needn't
    gpt.map_region(
        0x8000000,
        0x8000000,
        0x20000,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
    )?;

    if let Some(pcie) = meta.pcie {
        gpt.map_region(
            pcie.base_address,
            pcie.base_address,
            pcie.size,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
    }
    debug!("map pcie");

    for flash in meta.flash.iter() {
        gpt.map_region(
            flash.base_address,
            flash.base_address,
            flash.size,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
    }
    debug!("map flash");

    info!(
        "physical memory: [{:#x}: {:#x})",
        meta.physical_memory_offset,
        meta.physical_memory_offset + meta.physical_memory_size
    );
    gpt.map_region(
        meta.physical_memory_offset,
        meta.physical_memory_offset,
        meta.physical_memory_size,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
    )?;
    debug!("map physical memeory");

    gpt.map_region(
        NIMBOS_KERNEL_BASE_VADDR,
        kernel_entry,
        meta.physical_memory_size,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
    )?;

    Ok(gpt)
}