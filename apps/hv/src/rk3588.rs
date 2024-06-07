#![no_std]
#![no_main]
extern crate alloc;
#[macro_use] extern crate axstd;
use log::*;

const NIMBOS_DTB_SIZE: usize = 7522;
const NIMBOS_KERNEL_SIZE: usize = 552960;
// const NIMBOS_KERNEL_SIZE: usize = 292;
const NIMBOS_MEM_SIZE: usize = 0x80_0000;

#[link_section = ".guestdata.dtb"]
static NIMBOS_DTB: [u8; NIMBOS_DTB_SIZE] = *include_bytes!("../../guest/nimbos/nimbos-aarch64_rk3588.dtb");
#[link_section = ".guestdata.kernel"]
static NIMBOS_KERNEL: [u8; NIMBOS_KERNEL_SIZE] = *include_bytes!("../../guest/nimbos/nimbos-aarch64_rk3588.bin");
#[link_section = ".guestdata.mem"]
static NIMBOS_MEM: [u8; NIMBOS_MEM_SIZE] = [0; NIMBOS_MEM_SIZE];

extern "C" {
    fn __guest_dtb_start();
    fn __guest_dtb_end();
    fn __guest_kernel_start();
    fn __guest_kernel_end();
}



fn test_dtbdata() {
    // 地址转换为指针
    let address: *const u8 = NIMBOS_DTB.as_ptr() as usize as * const u8;

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

fn test_kerneldata() {
    // 地址转换为指针
    let address: *const u8 = NIMBOS_KERNEL.as_ptr() as usize as * const u8;

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

pub use arceos_hv::dtb_aarch64::*;
pub use arceos_hv::aarch64_config::*;

// mod dtb_aarch64;
// mod aarch64_config;

// mod dtb_aarch64::MachineMeta;
// mod aarch64_config::*;
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
// use dtb_aarch64;
// use aarch64_config;
use alloc::vec::Vec;
use page_table_entry::MappingFlags;

fn copy_high_data() -> usize {

    //  申请一块内存  大小为 memory 大小
    use alloc::alloc::Layout;
    let layout = Layout::from_size_align(NIMBOS_MEM_SIZE, 8192).unwrap();
    let area_base: *mut u8 = unsafe { alloc::alloc::alloc_zeroed(layout) };
    info!("base: {:#x}, layout size: {:#x}", area_base as usize, layout.size());

    let tls_load_base = __guest_kernel_start as *mut u8;
    let tls_load_size = __guest_kernel_end as usize - __guest_kernel_start as usize;
    unsafe {
        // copy data from .tbdata section
        core::ptr::copy_nonoverlapping(
            tls_load_base,
            area_base,
            tls_load_size,
        );
        use core::arch::asm;
        //asm!("isb");
    }

    area_base as usize
}

#[no_mangle] fn main(hart_id: usize) {
    println!("Hello, hv!");

    {
        // let vm1_kernel_entry = 0x7020_0000;
        // let vm1_dtb = 0x7000_0000;
        
        let vm1_kernel_entry = KERNEL_BASE_PADDR;
        let vm1_dtb = __guest_dtb_start as usize;
        test_kerneldata();
        test_dtbdata();

        // boot cpu
        PerCpu::<HyperCraftHalImpl>::init(0).unwrap(); 
        debug!("33");
        // get current percpu
        let percpu = PerCpu::<HyperCraftHalImpl>::ptr_for_cpu(hart_id);
        // create vcpu, need to change addr for aarch64!
        let gpt = setup_gpm(vm1_dtb).unwrap();  
        let vcpu = percpu.create_vcpu(0, 0).unwrap();
        percpu.set_active_vcpu(Some(vcpu.clone()));

        let vcpus = VcpusArray::new();
        debug!("42");

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
        debug!("54");
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
    
    let addr = copy_high_data();
    // 创建一个长度为10的数组来存储读取的数据
    let mut buffer = [0u8; 20];

    unsafe {
        // 从指定地址读取10个字节
        for i in 0..20 {
            buffer[i] = *(addr as *const u8).offset(i as isize);
        }
    }

    // 输出读取的数据
    debug!("{:?}", buffer);

    for (i,c)in meta.console.iter().enumerate() {
        gpt.map_region(
            c.base_address,
            c.base_address,
            c.size,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
        debug!("map console{i} : {:#x} -  {:#x}",c.base_address, c.size);
    }

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
    gpt.map_region (
        KERNEL_BASE_PADDR,
        kernel_entry,
        meta.physical_memory_size,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
    )?;

    Ok(gpt)
}