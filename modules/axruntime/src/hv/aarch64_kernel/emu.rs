use spin::Mutex;
extern crate alloc;
use alloc::vec::Vec;

use hypercraft::arch::emu::*;
use hypercraft::arch::utils::in_range;

use super::current_cpu;

pub const EMU_DEV_NUM_MAX: usize = 32;
pub static EMU_DEVS_LIST: Mutex<Vec<EmuDevEntry>> = Mutex::new(Vec::new());

// TO CHECK
pub fn emu_handler(emu_ctx: &EmuContext) -> bool {
    let ipa = emu_ctx.address;
    let emu_devs_list = EMU_DEVS_LIST.lock();

    for emu_dev in &*emu_devs_list {
        let active_vcpu = current_cpu().get_active_vcpu().unwrap();
        if active_vcpu.vm_id == emu_dev.vm_id && 
            in_range(ipa, emu_dev.ipa, emu_dev.size - 1) {
            let handler = emu_dev.handler;
            let id = emu_dev.id;
            drop(emu_devs_list);
            return handler(id, emu_ctx);
        }
    }
    debug!(
        "emu_handler: no emul handler for Core {} data abort ipa 0x{:x}",
        current_cpu().cpu_id,
        ipa
    );
    return false;
}

pub fn emu_register_dev( emu_type: EmuDeviceType, vm_id: usize, dev_id: usize, address: usize,
    size: usize, handler: EmuDevHandler,) 
{
    let mut emu_devs_list = EMU_DEVS_LIST.lock();
    if emu_devs_list.len() >= EMU_DEV_NUM_MAX {
        panic!("emu_register_dev: can't register more devs");
    }

    for emu_dev in &*emu_devs_list {
        if vm_id != emu_dev.vm_id {
            continue;
        }
        if in_range(address, emu_dev.ipa, emu_dev.size - 1)
            || in_range(emu_dev.ipa, address, size - 1)
        {
            panic!("emu_register_dev: duplicated emul address region: prev address 0x{:x} 
                size 0x{:x}, next address 0x{:x} size 0x{:x}", emu_dev.ipa, emu_dev.size, address, size);
        }
    }

    emu_devs_list.push(EmuDevEntry { emu_type, vm_id, id: dev_id, ipa: address, size, handler, });
}

pub fn emu_remove_dev(vm_id: usize, dev_id: usize, address: usize, size: usize) {
    let mut emu_devs_list = EMU_DEVS_LIST.lock();
    for (idx, emu_dev) in emu_devs_list.iter().enumerate() {
        if vm_id == emu_dev.vm_id
            && emu_dev.ipa == address
            && emu_dev.id == dev_id
            && emu_dev.size == size
        {
            emu_devs_list.remove(idx);
            return;
        }
    }
    panic!(
        "emu_remove_dev: emu dev not exist address 0x{:x} size 0x{:x}",
        address, size
    );
}
