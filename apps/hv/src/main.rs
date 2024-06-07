#![no_std]
#![no_main]
extern crate alloc;
#[macro_use] extern crate libax;

use dtb_aarch64::MachineMeta;
use aarch64_config::*;
use libax::info;
use libax::hv::{
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
    let vm2_kernel_entry = 0x5020_0000;
    let vm2_dtb = 0x5000_0000;
    
    PerCpu::<HyperCraftHalImpl>::setup_this_cpu(cpu_id).unwrap();
    let percpu = PerCpu::<HyperCraftHalImpl>::this_cpu();
    let virt_cpu = percpu.create_vcpu(1, 0).unwrap();
    percpu.set_active_vcpu(Some(virt_cpu.clone()));
    let vcpus = VcpusArray::new();

    let gpt = setup_gpm(vm2_dtb, vm2_kernel_entry).unwrap(); 
    let vm: VM<HyperCraftHalImpl, GuestPageTable> = VM::new(vcpus, gpt, 1).unwrap();

    add_vm(1, vm);
    let vcpu_id = virt_cpu.vcpu_id;
    add_vm_vcpu(1, virt_cpu);
    init_vm_vcpu(1, vcpu_id, vm2_kernel_entry, vm2_dtb);
    init_vm_emu_device(1);
    init_vm_passthrough_device(1);

    run_vm_vcpu(1, 0);
}

pub fn setup_gpm(dtb: usize, kernel_entry: usize) -> Result<GuestPageTable> {
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
    // gicv3 needn't
    gpt.map_region(
        0x8010000,
        0x8040000,
        0x2000,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
    )?;

    // ???
    gpt.map_region(
        0x8020000,
        0x8020000,
        0x10000,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
    )?;

    // gicv3 needn't
    gpt.map_region(
        0x8000000,
        0x8000000,
        0x10000,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
    )?;

    // v3 its nimbos needn't
    gpt.map_region(
        0x8080000,
        0x8080000,
        0x20000,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
    )?;

    debug!("map its");

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

    // let vaddr = 0x8010000;
    // let hpa = gpt.translate(vaddr)?;
    // debug!("translate vaddr: {:#x}, hpa: {:#x}", vaddr, hpa);

    gpt.map_region(
        NIMBOS_KERNEL_BASE_VADDR,
        kernel_entry,
        meta.physical_memory_size,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
    )?;

    Ok(gpt)
}

/*
qemu-system-aarch64 -m 3G -smp 2 -cpu cortex-a72 -machine virt -nographic   \
-machine virtualization=on,gic-version=2                                    \
-kernel apps/hv/hv_qemu-virt-aarch64.bin                                    \
-device loader,file=apps/hv/guest/linux/linux-aarch64.dtb,addr=0x70000000,force-raw=on \
-device loader,file=apps/hv/guest/linux/linux-aarch64.bin,addr=0x70200000,force-raw=on \
-drive if=none,file=apps/hv/guest/linux/rootfs-aarch64.img,format=raw,id=hd0           \
-device virtio-blk-device,drive=hd0                                                    \
-device loader,file=apps/hv/guest/linux/linux-aarch64_1.dtb,addr=0x50000000,force-raw=on \
-device loader,file=apps/hv/guest/linux/linux-aarch64_1.bin,addr=0x50200000,force-raw=on \
-drive if=none,file=apps/hv/guest/linux/rootfs-aarch64_1.img,format=raw,id=hd1           \
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

/*
[ 14.128743 0 axhal::arch::aarch64::hv::exception:66] IRQ routed to EL2!!!!!!!!!!!!!!!
[ 14.129414 0 axhal::platform::aarch64_common::gic:132] this is iar:0x1b
[ 14.129953 0 axhal::arch::aarch64::hv::exception:69] src 0 id27
[ 14.130434 0 axhal::trap:36] in handle_irq_extern_hv: irq_num 27, src 0
[ 14.130960 0 axruntime::hv::kernel::interrupt:8] src 0x0 id0x1b virtual interrupt

[ 14.131544 0 axruntime::hv::kernel::interrupt:53] [interrupt_vm_inject] this is interrupt vm inject
[ 14.132219 0 axruntime::hv::kernel::interrupt:58] [interrupt_vm_inject] before vgic_inject
[ 14.132818 0 axruntime::hv::kernel::vgic:870] [vgic_inject] Core 0 inject int 27 to vm0
[ 14.133423 0 axruntime::hv::kernel::vgic:874] [vgic_inject] interrupt is hw
[ 14.134008 0 axruntime::hv::kernel::vgic:383] [route]
[ 14.134481 0 axruntime::hv::kernel::vgic:395] route: int_targets 0x1, irq: 27
[ 14.135061 0 axruntime::hv::kernel::vgic:177] [add_lr] irq:27, target 1
[ 14.135585 0 axruntime::hv::kernel::vgic:184] [add_lr]  this is gic_lr number 4
[ 14.136137 0 axruntime::hv::kernel::vgic:190] [add_lr] elrsr0: 0xf
[ 14.136637 0 axruntime::hv::kernel::vgic:198] [add_lr] this is lr_idx Some(0)
[ 14.137184 0 axruntime::hv::kernel::vgic:285] write lr: lr_idx 0 vcpu_id:0, int_id:27, int_prio:160
[ 14.137868 0 axruntime::hv::kernel::vgic:288] write lr: prev_int_id 27
[ 14.138404 0 axruntime::hv::kernel::vgic:311] write lr: interrupt state 1
[ 14.138923 0 axruntime::hv::kernel::vgic:316] write lr: this is hw interrupt
[ 14.139521 0 axruntime::hv::kernel::vgic:375] write lr: lr value 0x9a006c1b
[ 14.140090 0 axruntime::hv::kernel::vgic:379] write lr: end
[ 14.140577 0 axruntime::hv::kernel::interrupt:60] [interrupt_vm_inject] after vm 0 inject irq 27
[ 14.141258 0 axruntime::hv::kernel::interrupt:49] interrupt_handler: core 0 receive virtual int 27

[ 14.141962 0 axruntime::trap:29] [handle_irq_hv] before deactivate irq 27 
[ 14.142527 0 axhal::platform::aarch64_common::gic:180] gicc_clear_current_irq: irq 27, for_hypervisor false
[ 14.143278 0 axruntime::trap:37] [handle_irq_hv] after deactivate irq 27
*/


/*
[0x40080000,   0x400ab000) .text          (READ | EXECUTE | RESERVED)
[0x400ab000,   0x400b2000) .rodata        (READ | RESERVED)
[0x400b2000,   0x400b5000) .data          (READ | WRITE | RESERVED)
[0x400b5000,   0x400b6000) .percpu        (READ | WRITE | RESERVED)
[0x400b6000,   0x400f6000) boot stack     (READ | WRITE | RESERVED)
[0x400f6000,   0x4011c000) .bss           (READ | WRITE | RESERVED)

[0x4011 c000,   0x48000000) free memory    (READ | WRITE | FREE)
                                           
[0x9000000,    0x9001000)    mmio         (READ | WRITE | DEVICE | RESERVED)   ---pl011 addr: 0x9000000, size: 0x1000
[0x8000000,    0x8040000)    mmio         (READ | WRITE | DEVICE | RESERVED)   intc addr: 0x8000000, size: 0x10000   addr: 0x8010000, size: 0x10000  addr:0x8020000, size: 0x1000
[0xa000000,    0xa004000)    mmio         (READ | WRITE | DEVICE | RESERVED)   vm map virtio
[0x10000000,   0x3eff0000)   mmio         (READ | WRITE | DEVICE | RESERVED)
[0x4010000000, 0x4020000000) mmio         (READ | WRITE | DEVICE | RESERVED)


initialize global allocator at: [0x4011_c000, 0x4800_0000)

*/

/*
cd existing_repo
git remote rename origin old-origin
git remote add origin https://gitlab.eduxiji.net/T202410054992503/project2210132-222878.git
git push -u origin --all
git push -u origin --tags
@T202410054992503
T202410054992503@eduxiji.net
ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQCJqdP6XHoRzvEivVIsRIIb3EVrhe3OQmS0YE8KteOUidE0SXt8nZokGQSyKzx3mCNynNW2Db7nbAG6r8mMzYKx1IbvzBJzZDIjoBdQeCGJru/NH+CMs3zI8SfsLdQz43ieLRw9y4NXWA3P1oFuanwGuCgl9Zl96lSXaSUvT4sHTg46At1VgSddODQXjz6BoyTgnIai5c7OCVkpS5IUEKDl/Z5i2TwcD/5lKL4rZy5MRmKCUDw41i9n8EhPnCpSIAqv1zVRhj4hxZTfDMOUiBy0/aQ3KJW1Dws0YyuqDeMOsfgVTKBxywVQedc09wkGbeKS93OCPsHLL78cbquOwmCjUT/j3UVLHbUZQAqhd8mWgjlvtSsUu6ga6Xcst5mptf1cCmIcNy41CIzxZlnS+h6CHdizE27CHlpgFcoQUBnnTKMSRBMNNFfTxwsWDf9npvHA6AvFk00LjvZOGGFL5892IVV29QOYh+70eGN2GUUePV5MH48tF8A8hkFFaaMhw/0= 927068267@qq.com
*/

/*
cp ./nimbos.bin ~/cicv/arceos/apps/hv/guest/nimbos/ && rm ~/cicv/arceos/apps/hv/guest/nimbos/nimbos-aarch64-v3.bin && mv ~/cicv/arceos/apps/hv/guest/nimbos/nimbos.bin ~/cicv/arceos/apps/hv/guest/nimbos/nimbos-aarch64-v3.bin
*/