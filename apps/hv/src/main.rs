#![no_std]
#![no_main]
extern crate alloc;
#[macro_use] extern crate axstd;
use log::*;

use dtb_aarch64::MachineMeta;
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
use alloc::vec::Vec;
use page_table_entry::MappingFlags;

extern "C" {
    fn guestdtb_start();
    fn guestdtb_end();
    fn guestkernel_start();
    fn guestkernel_end();
}

core::arch::global_asm!(include_str!("../guest.S"));

fn copy_data(src: *mut u8, dst:  *mut u8, size: usize) {
    unsafe {
        // copy data from .tbdata section
        core::ptr::copy_nonoverlapping(
            src,
            dst,
            size,
        );
    }
}

fn test_dtbdata_high() {
    // 地址转换为指针
    let address: *const u8 = 0x7000_0000 as * const u8;

    // 创建一个长度为10的数组来存储读取的数据
    let mut buffer = [0u8; 20];

    unsafe {
        // 从指定地址读取10个字节
        for i in 0..20 {
            buffer[i] = *address.offset(i as isize);
        }
    }

    // 输出读取的数据
    debug!("{:?}", buffer);
}

/*
 * 运行需要nimbos启用gicv3
 * 
*/
#[no_mangle] fn main(hart_id: usize) {
    println!("Hello, hv!");
    {
        // qemu-virt
        let vm1_dtb: usize = axconfig::GUEST1_PHYSMEM_START;
        let vm1_kernel_entry: usize = vm1_dtb + axconfig::GUEST1_KERNEL_OFFSET;
        
        let dtb_start_addr = guestdtb_start as usize;
        let kernel_start_addr = guestkernel_start as usize;
        unsafe {
            copy_data(dtb_start_addr as *mut u8, vm1_dtb as *mut u8, 0x20_0000);
            copy_data(kernel_start_addr as *mut u8, vm1_kernel_entry as *mut u8, 0x40_0000);
        }

        // boot cpu 
        PerCpu::<HyperCraftHalImpl>::init(0).unwrap(); 
        // get current percpu
        let percpu = PerCpu::<HyperCraftHalImpl>::ptr_for_cpu(hart_id);
        // create vcpu, need to change addr for aarch64!
        let gpt = setup_gpm(vm1_dtb).unwrap();  
        let vcpu: axstd::hv::VCpu<HyperCraftHalImpl> = percpu.create_vcpu(0, 0).unwrap();
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

pub fn setup_gpm(dtb: usize) -> Result<GuestPageTable> {
    let mut gpt = GuestPageTable::new()?;
    let meta = MachineMeta::parse(dtb);

    for (i,c)in meta.console.iter().enumerate() {
        gpt.map_region(
            c.base_address,
            axconfig::UART_PADDR,
            c.size,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
        debug!("map console{i} :{:#x}-to {:#x}  {:#x}",c.base_address, axconfig::UART_PADDR,c.size);
    }
    
    if let Some(pcie) = meta.pcie {
        gpt.map_region(
            pcie.base_address,
            pcie.base_address,
            pcie.size,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
        debug!("map pcie : {:#x} to {:#x}", pcie.base_address,pcie.base_address);
    }

    for flash in meta.flash.iter() {
        gpt.map_region(
            flash.base_address,
            flash.base_address,
            flash.size,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
        debug!("map flash : {:#x} to {:#x}", flash.base_address,flash.base_address);
    }

    info!(
        "physical memory: [{:#x}: {:#x})",
        meta.physical_memory_offset,
        meta.physical_memory_offset + meta.physical_memory_size
    );

    gpt.map_region(
        meta.physical_memory_offset,
        axconfig::GUEST1_PHYSMEM_START,
        meta.physical_memory_size,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
    )?;

    debug!("map physical_memory: {:#x} to {:#x} size {:#x}", meta.physical_memory_offset,axconfig::GUEST1_PHYSMEM_START,
        meta.physical_memory_size);
    Ok(gpt)
}
