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
static NIMBOS_DTB: [u8; NIMBOS_DTB_SIZE] = *include_bytes!("../../guest/nimbos/nimbos-aarch64.dtb");
#[link_section = ".guestdata.kernel"]
static NIMBOS_KERNEL: [u8; NIMBOS_KERNEL_SIZE] = *include_bytes!("../../guest/nimbos/nimbos-aarch64.bin");
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
        
        let vm1_kernel_entry = NIMBOS_KERNEL_BASE_PADDR;
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

    gpt.map_region(
        NIMBOS_KERNEL_BASE_PADDR,
        addr,
        NIMBOS_MEM_SIZE,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
    )?;

    // // Ok(gpt)
    gpt.map_region( 0x900_0000, 0xFEB50000, 0x1000,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
    ) ?;
    debug!("map virtio");   // ok
     

    gpt.map_region(
        0x7000_0000,
        0x7000_0000,
        NIMBOS_MEM_SIZE,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
    ) ?;
    debug!("map physical memeory");
    // nimbos memory
    gpt.map_region(
        0x8000_0000,
        0x8000_0000,
        NIMBOS_MEM_SIZE,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
    ) ?;
    debug!("map physical memeory");

    // 任意的 kernel entry 都能映射到 439000
    // 将 0000_0000_4008_0000 映射到kernel img地址
    // gpt.map_region(
    //     0x43a000,
    //     0x43a000,
    //     NIMBOS_MEM_SIZE,
    //     MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
    // ) ?;
    
    // 将 ffff_0000_4008_0000 映射到kernel img地址
    // gpt.map_region(
    //     NIMBOS_KERNEL_BASE_VADDR,
    //     __guest_kernel_start as usize,
    //     NIMBOS_MEM_SIZE,
    //     MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
    // ) ?;

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
qemu-system-aarch64 -m 3G -smp 1 -cpu cortex-a72 -machine virt -nographic
-machine virtualization=on,gic-version=2  
-kernel apps/hv/hv_aarch64-qemu-virt-hv.bin 
-device loader,file=apps/hv/guest/linux/linux-aarch64.dtb,addr=0x70000000,force-raw=on 
-device loader,file=apps/hv/guest/linux/linux-aarch64.bin,addr=0x70200000,force-raw=on 
-drive id=hd0,if=none,file=apps/hv/guest/linux/rootfs-aarch64.img,format=raw
-device virtio-blk-device,drive=hd0 

-device virtio-blk-device,drive=disk0 
-drive id=disk0,if=none,format=raw,file=disk.img 
-device virtio-net-device,netdev=net0 
-netdev user,id=net0,hostfwd=tcp::5555-:5555,hostfwd=udp::5555-:5555 
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

*/