#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate libax;

#[cfg(target_arch = "riscv64")]
use dtb_riscv64::MachineMeta;
#[cfg(target_arch = "aarch64")]
use dtb_aarch64::MachineMeta;
#[cfg(target_arch = "aarch64")]
use aarch64_config::*;
#[cfg(target_arch = "aarch64")]
use libax::{
    hv::{
        self, GuestPageTable, GuestPageTableTrait, HyperCraftHalImpl, PerCpu,
        Result, VCpu, VmCpus, VM, VcpusArray, 
        VM_ARRAY, VM_MAX_NUM,
        add_vm, add_vm_vcpu, get_vm, print_vm,
        init_vm_vcpu, init_vm_emu_device, init_vm_passthrough_device, 
        is_vcpu_init_ok, is_vcpu_primary_ok,
        run_vm_vcpu, 
    },
    info,
};
#[cfg(not(target_arch = "aarch64"))]
use libax::{
    hv::{
        self, GuestPageTable, GuestPageTableTrait, HyperCallMsg, HyperCraftHalImpl, PerCpu, Result,
        VCpu, VmCpus, VmExitInfo, VM, phys_to_virt,
    },
    info,
};

use page_table_entry::MappingFlags;

#[cfg(target_arch = "riscv64")]
mod dtb_riscv64;
#[cfg(target_arch = "aarch64")]
mod dtb_aarch64;
#[cfg(target_arch = "aarch64")]
mod aarch64_config;
#[cfg(target_arch = "aarch64")]
use alloc::vec::Vec;

#[cfg(target_arch = "x86_64")]
mod x64;

#[no_mangle] fn main(hart_id: usize) {
    println!("Hello, hv!");

    #[cfg(target_arch = "riscv64")]
    {
        // boot cpu
        PerCpu::<HyperCraftHalImpl>::init(0, 0x4000);

        // get current percpu
        let pcpu = PerCpu::<HyperCraftHalImpl>::this_cpu();

        // create vcpu
        let gpt = setup_gpm(0x9000_0000).unwrap();
        let vcpu = pcpu.create_vcpu(0, 0x9020_0000).unwrap();
        let mut vcpus = VmCpus::new();

        // add vcpu into vm
        vcpus.add_vcpu(vcpu).unwrap();
        let mut vm: VM<HyperCraftHalImpl, GuestPageTable> = VM::new(vcpus, gpt).unwrap();
        vm.init_vcpu(0);

        // vm run
        info!("vm run cpu{}", hart_id);
        vm.run(0);
    }
    #[cfg(target_arch = "aarch64")]
    {
        let vm1_kernel_entry = 0x7020_0000;
        let vm1_dtb = 0x7000_0000;

        // boot cpu
        PerCpu::<HyperCraftHalImpl>::init(0); 
        // get current percpu
        let percpu = PerCpu::<HyperCraftHalImpl>::ptr_for_cpu(hart_id);
        // create vcpu, need to change addr for aarch64!
        let gpt = setup_gpm(vm1_dtb, vm1_kernel_entry).unwrap();  
        let vcpu = percpu.create_vcpu(0, 0).unwrap();
        percpu.set_active_vcpu(Some(vcpu.clone()));

        let vcpus = VcpusArray::new();

        // add vcpu into vm
        let mut vm: VM<HyperCraftHalImpl, GuestPageTable> = VM::new(vcpus, gpt, 0).unwrap();
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
    #[cfg(target_arch = "x86_64")]
    {
        println!("into main {}", hart_id);

        let mut p = PerCpu::<HyperCraftHalImpl>::new(hart_id);
        p.hardware_enable().unwrap();

        let gpm = x64::setup_gpm().unwrap();
        info!("{:#x?}", gpm);

        let mut vcpu = p
            .create_vcpu(x64::BIOS_ENTRY, gpm.nest_page_table_root())
            .unwrap();

        println!("Running guest...");
        vcpu.run();

        p.hardware_disable().unwrap();

        return;
    }
    #[cfg(not(any(target_arch = "riscv64", target_arch = "x86_64", target_arch = "aarch64")))]
    {
        panic!("Other arch is not supported yet!")
    }
}


/*
#[cfg(target_arch = "riscv64")] pub fn setup_gpm(dtb: usize) -> Result<GuestPageTable> {
    let mut gpt = GuestPageTable::new()?;
    let meta = MachineMeta::parse(dtb);
    if let Some(test) = meta.test_finisher_address {
        gpt.map_region(
            test.base_address,
            test.base_address,
            test.size + 0x1000,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER | MappingFlags::EXECUTE,
        )?;
    }
    for virtio in meta.virtio.iter() {
        gpt.map_region(
            virtio.base_address,
            virtio.base_address,
            virtio.size,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
    }

    if let Some(uart) = meta.uart {
        gpt.map_region(
            uart.base_address,
            uart.base_address,
            0x1000,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
    }

    if let Some(clint) = meta.clint {
        gpt.map_region(
            clint.base_address,
            clint.base_address,
            clint.size,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
    }

    if let Some(plic) = meta.plic {
        gpt.map_region(
            plic.base_address,
            plic.base_address,
            0x20_0000,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
    }

    if let Some(pci) = meta.pci {
        gpt.map_region(
            pci.base_address,
            pci.base_address,
            pci.size,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
    }

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

    Ok(gpt)
}
*/

/*
qemu-system-aarch64 -m 3G -smp 2 -cpu cortex-a72 -machine virt -nographic   \
-machine virtualization=on,gic-version=2                                    \
-kernel apps/hv/hv_qemu-virt-aarch64.bin                                    \
-device loader,file=apps/hv/guest/linux/linux-aarch64.dtb,addr=0x70000000,force-raw=on \
-device loader,file=apps/hv/guest/linux/linux-aarch64.bin,addr=0x70200000,force-raw=on \
-drive if=none,file=apps/hv/guest/linux/rootfs-aarch64.img,format=raw,id=hd0           \
-device virtio-blk-device,drive=hd0                                                    \
-device loader,file=apps/hv/guest/linux/linux-aarch64-1.dtb,addr=0x50000000,force-raw=on \
-device loader,file=apps/hv/guest/linux/linux-aarch64-1.bin,addr=0x50200000,force-raw=on \
-drive if=none,file=apps/hv/guest/linux/rootfs-aarch64-1.img,format=raw,id=hd1           \
-device virtio-blk-device,drive=hd1                                                     
*/

/*
qemu-system-aarch64 -m 3G -smp 2 -cpu cortex-a72 -machine virt -nographic   \
-machine virtualization=on,gic-version=2                                    \
-kernel apps/hv/hv_qemu-virt-aarch64.bin                                    \
-device loader,file=apps/hv/guest/nimbos/nimbos-aarch64_1.dtb,addr=0x70000000,force-raw=on    \
-device loader,file=apps/hv/guest/nimbos/nimbos-aarch64_1.bin,addr=0x70200000,force-raw=on    \
-device loader,file=apps/hv/guest/nimbos/nimbos-aarch64.dtb,addr=0x50000000,force-raw=on      \
-device loader,file=apps/hv/guest/nimbos/nimbos-aarch64.bin,addr=0x50200000,force-raw=on 
*/

#[cfg(target_arch = "aarch64")] #[no_mangle] pub extern "C" fn secondary_vm(cpu_id: usize) ->! {
    while !is_vcpu_primary_ok() {
        core::hint::spin_loop();
    }
    let vm2_kernel_entry = 0x5020_0000;
    let vm2_dtb = 0x5000_0000;
    
    PerCpu::<HyperCraftHalImpl>::setup_this_cpu(cpu_id);
    let percpu = PerCpu::<HyperCraftHalImpl>::this_cpu();
    let vcpu = percpu.create_vcpu(1, 0).unwrap();
    // create vcpu, need to change addr for aarch64!
    let gpt = setup_gpm(vm2_dtb, vm2_kernel_entry).unwrap();  
    percpu.set_active_vcpu(Some(vcpu.clone()));
    let vcpus = VcpusArray::new();
    // add vcpu into vm
    // vcpus.add_vcpu(vcpu).unwrap();
    let mut vm: VM<HyperCraftHalImpl, GuestPageTable> = VM::new(vcpus, gpt, 1).unwrap();

    add_vm(1, vm);
    let vcpu_id = vcpu.vcpu_id;
    add_vm_vcpu(1, vcpu);
    init_vm_vcpu(1, vcpu_id, vm2_kernel_entry, vm2_dtb);
    init_vm_emu_device(1);
    init_vm_passthrough_device(1);

    run_vm_vcpu(1, 0);
}

#[cfg(target_arch = "aarch64")] pub fn setup_gpm(dtb: usize, kernel_entry: usize) -> Result<GuestPageTable> {
    let mut gpt = GuestPageTable::new()?;
    let meta = MachineMeta::parse(dtb);
    /* 
    for virtio in meta.virtio.iter() {
        gpt.map_region(
            virtio.base_address,
            virtio.base_address,
            0x1000, 
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
        debug!("finish one virtio");
    }
    */
    // hard code for virtio_mmio
    gpt.map_region(
        0xa000000,
        0xa000000,
        0x4000,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
    )?;
    debug!("map virtio");
    
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

    /* 
    for intc in meta.intc.iter() {
        gpt.map_region(
            intc.base_address,
            intc.base_address,
            intc.size,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        )?;
    }
    */
    // map gicc to gicv. the address is qemu setting, it is different from real hardware
    gpt.map_region(
        0x8010000,
        0x8040000,
        0x2000,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
    )?;

    gpt.map_region(
        0x8020000,
        0x8020000,
        0x10000,
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

    let vaddr = 0x8010000;
    let hpa = gpt.translate(vaddr)?;
    debug!("translate vaddr: {:#x}, hpa: {:#x}", vaddr, hpa);

    gpt.map_region(
        NIMBOS_KERNEL_BASE_VADDR,
        kernel_entry,
        meta.physical_memory_size,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
    )?;

    Ok(gpt)
}
