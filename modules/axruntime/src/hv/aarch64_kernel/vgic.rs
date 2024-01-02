use arm_gic::{
    GIC_CONFIG_BITS, GIC_PRIO_BITS, GIC_PRIVATE_INT_NUM, GIC_SGIS_NUM, GIC_TARGETS_MAX,
    GIC_TARGET_BITS,
};
use axhal::{gic_is_priv, gic_lrs, GICD, GICH, GICV};
use hypercraft::arch::emu::EmuContext;
use hypercraft::arch::utils::*;
use hypercraft::arch::vgic::{Vgic, VgicInt, VgicIntInner};
use hypercraft::{IrqState, VCpu, VM};

use crate::{GuestPageTable, HyperCraftHalImpl};
use hypercraft::{GuestPageTableTrait, HyperCraftHal};

use super::vm_array::get_vm;
use super::{active_vm, current_cpu};

fn remove_int_list(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>, is_pend: bool) {
    let mut cpu_priv = vgic.cpu_priv.lock();
    let vcpu_id = vcpu.vcpu_id;
    let int_id = interrupt.id();
    if is_pend {
        if !interrupt.in_pend() {
            return;
        }
        for i in 0..cpu_priv[vcpu_id].pend_list.len() {
            if cpu_priv[vcpu_id].pend_list[i].id() == int_id {
                // if int_id == 297 {
                //     println!("remove int {} in pend list[{}]", int_id, i);
                // }
                cpu_priv[vcpu_id].pend_list.remove(i);
                break;
            }
        }
        interrupt.set_in_pend_state(false);
    } else {
        if !interrupt.in_act() {
            return;
        }
        for i in 0..cpu_priv[vcpu_id].act_list.len() {
            if cpu_priv[vcpu_id].act_list[i].id() == int_id {
                cpu_priv[vcpu_id].act_list.remove(i);
                break;
            }
        }
        interrupt.set_in_act_state(false);
    };
}

fn add_int_list(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>, is_pend: bool) {
    let mut cpu_priv = vgic.cpu_priv.lock();
    let vcpu_id = vcpu.vcpu_id;
    if is_pend {
        interrupt.set_in_pend_state(true);
        cpu_priv[vcpu_id].pend_list.push_back(interrupt);
    } else {
        interrupt.set_in_act_state(true);
        cpu_priv[vcpu_id].act_list.push_back(interrupt);
    }
}

/// update vgic int list according to the coming interrupt state
fn update_int_list(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) {
    let state = interrupt.state().to_num();

    // if state is pending and the interrupt is not pending, add it to the pending list
    // bool means is_pend
    if state & 1 != 0 && !interrupt.in_pend() {
        add_int_list(vgic, vcpu.clone(), interrupt.clone(), true);
    } else if state & 1 == 0 {
        remove_int_list(vgic, vcpu.clone(), interrupt.clone(), true);
    }
    // if state is active and the interrupt is not active, add it to the active list
    if state & 2 != 0 && !interrupt.in_act() {
        add_int_list(vgic, vcpu.clone(), interrupt.clone(), false);
    } else if state & 2 == 0 {
        remove_int_list(vgic, vcpu.clone(), interrupt.clone(), false);
    }

    if interrupt.id() < GIC_SGIS_NUM as u16 {
        if vgic.cpu_priv_sgis_pend(vcpu.vcpu_id, interrupt.id() as usize) != 0
            && !interrupt.in_pend()
        {
            add_int_list(vgic, vcpu, interrupt, true);
        }
    }
}

/// Get vgic int list head
fn int_list_head(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, is_pend: bool) -> Option<VgicInt<HyperCraftHalImpl, GuestPageTable>> {
    let cpu_priv = vgic.cpu_priv.lock();
    let vcpu_id = vcpu.vcpu_id;
    if is_pend {
        if cpu_priv[vcpu_id].pend_list.is_empty() {
            None
        } else {
            Some(cpu_priv[vcpu_id].pend_list[0].clone())
        }
    } else {
        if cpu_priv[vcpu_id].act_list.is_empty() {
            None
        } else {
            Some(cpu_priv[vcpu_id].act_list[0].clone())
        }
    }
}

fn get_int(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize) -> Option<VgicInt<HyperCraftHalImpl, GuestPageTable>> {
    if int_id < GIC_PRIVATE_INT_NUM {
        let vcpu_id = vcpu.vcpu_id;
        return Some(vgic.cpu_priv_interrupt(vcpu_id, int_id));
    } else if int_id >= GIC_PRIVATE_INT_NUM && int_id < 1024 {
        // hard code for max irq
        return Some(vgic.vgicd_interrupt(int_id - GIC_PRIVATE_INT_NUM));
    }
    return None;
}

// if the interrupt is invalid, just remove it, otherwise, according to the interrupt state, update the vgic pending or active list
fn remove_lr(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) -> bool {
    if !vgic_owns(vcpu.clone(), interrupt.clone()) {
        return false;
    }
    let int_lr_idx = interrupt.lr();
    let int_id = interrupt.id() as usize;
    let vcpu_id = vcpu.vcpu_id;

    if !interrupt.in_lr() {
        return false;
    }

    // set gich_lr[int_lr_idx] to 0, save the origin lr value to lv_val
    let mut lr_val = 0;
    if let Some(lr) = gich_get_lr(interrupt.clone()) {
        GICH.set_lr_by_idx(int_lr_idx as usize, 0);
        lr_val = lr;
    }
    // set this interrupt not in lr
    interrupt.set_in_lr(false);

    let lr_state = (lr_val >> 28) & 0b11;
    // if the origin lr value state is not invalid(pending or active), reset interrupt state
    if lr_state != 0 {
        interrupt.set_state(IrqState::num_to_state(lr_state as usize));
        if int_id < GIC_SGIS_NUM {
            // if interrupt is in active state, add it to cpu_priv active list
            if interrupt.state().to_num() & 2 != 0 {
                vgic.set_cpu_priv_sgis_act(vcpu_id, int_id, ((lr_val >> 10) & 0b111) as u8);
            }
            // if interrupt is in pending state, add it to cpu_priv pending list
            // ((lr_val >> 10) & 0b111) is the target cpu id
            else if interrupt.state().to_num() & 1 != 0 {
                let pend = vgic.cpu_priv_sgis_pend(vcpu_id, int_id);
                vgic.set_cpu_priv_sgis_pend(
                    vcpu_id,
                    int_id,
                    pend | (1 << ((lr_val >> 10) & 0b111) as u8),
                );
            }
        }
        // add this interrupt to the corresponding list
        update_int_list(vgic, vcpu, interrupt.clone());

        // if int is pending, signal a maintenance interrupt
        if (interrupt.state().to_num() & 1 != 0) && interrupt.enabled() {
            // debug!("remove_lr: interrupt_state {}", interrupt.state().to_num());
            let hcr = GICH.get_hcr();
            GICH.set_hcr(hcr | (1 << 3));
        }
        return true;
    }
    false
}

fn add_lr(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) -> bool {
    // Check if the interrupt is enabled and not already in the List Register.
    // If either condition is not met, return false.
    debug!("[add_lr] irq:{}, target {}", interrupt.id(), interrupt.targets());
    if !interrupt.enabled() || interrupt.in_lr() {
        return false;
    }

    // Get the number of List Registers.
    let gic_lrs = gic_lrs();
    debug!("[add_lr]  this is gic_lr number {}", gic_lrs);
    let mut lr_idx = None;
    // Find an empty slot in the List Registers.
    // elrsr: The corresponding List register does not contain a valid interrupt.

    let elrsr0 = GICH.get_elrsr_by_idx(0);
    debug!("[add_lr] elrsr0: {:#x}", elrsr0);

    for i in 0..gic_lrs {
        if (GICH.get_elrsr_by_idx(i / 32) & (1 << (i % 32))) != 0 {
            lr_idx = Some(i);
            break;
        }
    }
    debug!("[add_lr] this is lr_idx {:?}", lr_idx);
    // If no empty slot was found, we need to spill an existing interrupt.
    if lr_idx.is_none() {
        // Initialize variables to keep track of the interrupt with the lowest priority.
        let mut pend_found = 0;
        let mut act_found = 0;
        let mut min_prio_act = 0;
        let mut min_prio_pend = 0;
        let mut act_ind = None;
        let mut pend_ind = None;
        // Iterate over all List Registers to find the interrupt with the lowest priority.
        for i in 0..gic_lrs {
            let lr = GICH.get_lr_by_idx(i);
            // [27:23] Priority The priority of this interrupt.
            let lr_prio = (lr >> 23) & 0b11111;
            // [29:28] State The state of the interrupt.
            let lr_state = (lr >> 28) & 0b11;
            // Check if the interrupt is active.
            if lr_state & 2 != 0 {
                if lr_prio > min_prio_act {
                    min_prio_act = lr_prio;
                    act_ind = Some(i);
                }
                act_found += 1;
            }
            // Check if the interrupt is pending.
            else if lr_state & 1 != 0 {
                if lr_prio > min_prio_pend {
                    min_prio_pend = lr_prio;
                    pend_ind = Some(i);
                }
                pend_found += 1;
            }
        }
        // Choose the interrupt to spill based on the number of active and pending interrupts. First spill pending interrupts, then active interrupts.
        if pend_found > 1 {
            lr_idx = pend_ind;
        } else if act_found > 1 {
            lr_idx = act_ind;
        }
        // If an interrupt was chosen to be spilled, remove it from the List Register and yield its ownership.
        if let Some(idx) = lr_idx {
            let spilled_int = get_int(
                    vgic,
                    vcpu.clone(),
                    GICH.get_lr_by_idx(idx) as usize & 0b11_1111_1111,
                )
                .unwrap();
            // If the interrupt that we're going to spill is not the same as the interrupt we're trying to add,
            // lock the spilled interrupt to prevent other threads from modifying it while we're working with it.
            let spilled_int_lock;
            if spilled_int.id() != interrupt.id() {
                spilled_int_lock = spilled_int.lock.lock();
            }
            remove_lr(vgic, vcpu.clone(), spilled_int.clone());
            vgic_int_yield_owner(vcpu.clone(), spilled_int.clone());
            // if spilled_int.id() != interrupt.id() {
            //     drop(spilled_int_lock);
            // }
        }
    }

    // If an empty slot was found or an interrupt was spilled, write the new interrupt to the List Register.
    // Otherwise, if the interrupt is pending, enable maintenance interrupts.
    match lr_idx {
        Some(idx) => {
            write_lr(vgic, vcpu, interrupt, idx);
            return true;
        }
        None => {
            // turn on maintenance interrupts
            if vgic_get_state(interrupt) & 1 != 0 {
                let hcr = GICH.get_hcr();
                GICH.set_hcr(hcr | (1 << 3));
            }
        }
    }

    false
}

fn write_lr(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>, lr_idx: usize) {
    
    // Get the ID and priority of the vCPU and the interrupt.
    let vcpu_id = vcpu.vcpu_id;
    let int_id = interrupt.id() as usize;
    let int_prio = interrupt.get_priority();
    debug!("write lr: lr_idx {} vcpu_id:{}, int_id:{}, int_prio:{}", lr_idx, vcpu_id, int_id, int_prio);
    // Get the ID of the interrupt that is currently in the List Register.
    let prev_int_id = vgic.cpu_priv_curr_lrs(vcpu_id, lr_idx) as usize;
    debug!("write lr: prev_int_id {}", prev_int_id);
    // If the current interrupt is not the same as the interrupt we're trying to add,
    // we need to remove the current interrupt from the List Register.
    // This may happen if there is no empty slot in the List Registers and we need to spill an existing interrupt.
    if prev_int_id != int_id {
        let prev_interrupt_option = get_int(vgic, vcpu.clone(), prev_int_id);
        if let Some(prev_interrupt) = prev_interrupt_option {
            let prev_interrupt_lock = prev_interrupt.lock.lock();
            if vgic_owns(vcpu.clone(), prev_interrupt.clone()) {
                if prev_interrupt.in_lr() && prev_interrupt.lr() == lr_idx as u16 {
                    prev_interrupt.set_in_lr(false);
                    let prev_id = prev_interrupt.id() as usize;
                    if !gic_is_priv(prev_id) {
                        vgic_int_yield_owner(vcpu.clone(), prev_interrupt.clone());
                    }
                }
            }
            drop(prev_interrupt_lock);
        }
    }

    // Get the state of the interrupt and initialize the List Register value.
    let state = vgic_get_state(interrupt.clone());
    debug!("write lr: interrupt state {}", state);
    let mut lr = (int_id & 0b11_1111_1111) | (((int_prio as usize >> 3) & 0b1_1111) << 23);

    // If the interrupt is a hardware interrupt, set the appropriate bits in the List Register.
    if vgic_int_is_hw(interrupt.clone()) {
        debug!("write lr: this is hw interrupt");
        // [31] HW Indicates whether this virtual interrupt is a hardware interrupt meaning that it corresponds to a physical interrupt.
        lr |= 1 << 31;
        // When GICH_LR.HW is set to 1, this field indicates the physical interrupt ID that the hypervisor forwards to the Distributor.
        lr |= (0b11_1111_1111 & int_id) << 10;
        // 0b11: pending and active.
        if state == 3 {
            lr |= (2 & 0b11) << 28;
        } else {
            lr |= (state & 0b11) << 28;
        }
        let gicd = GICD.lock();
        if gicd.get_state(int_id) != 2 {
            gicd.set_state(int_id, 2, current_cpu().cpu_id);
        }
    }
    // If the interrupt is a software-generated interrupt (SGI), set the appropriate bits in the List Register.
    else if int_id < GIC_SGIS_NUM {
        // active state
        if (state & 2) != 0 {
            // ((vgic.cpu_priv_sgis_act(vcpu_id, int_id) as usize) << 10) & (0b111 << 10): cpu id
            lr |= ((vgic.cpu_priv_sgis_act(vcpu_id, int_id) as usize) << 10) & (0b111 << 10);
            // set active.
            lr |= (2 & 0b11) << 28;
        }
        // not active
        else {
            let mut idx = GIC_TARGETS_MAX - 1;
            // Loop through the targets of the SGI find target cpu id
            while idx as isize >= 0 {
                if (vgic.cpu_priv_sgis_pend(vcpu_id, int_id) & (1 << idx)) != 0 {
                    lr |= (idx & 0b111) << 10;
                    let pend = vgic.cpu_priv_sgis_pend(vcpu_id, int_id);
                    // clear the cpu idx corresponding pending bit
                    vgic.set_cpu_priv_sgis_pend(vcpu_id, int_id, pend & !(1 << idx));
                    lr |= (1 & 0b11) << 28;
                    break;
                }
                idx -= 1;
            }
        }
        // [19] EOI Indicates whether this interrupt triggers an EOI maintenance interrupt,
        // 1: A maintenance interrupt is asserted to signal EOI when the interrupt state is invalid, which typically occurs when the interrupt is deactivated.
        if vgic.cpu_priv_sgis_pend(vcpu_id, int_id) != 0 {
            lr |= 1 << 19;
        }
    } else {
        if !gic_is_priv(int_id) && !vgic_int_is_hw(interrupt.clone()) {
            lr |= 1 << 19;
        }

        lr |= (state & 0b11) << 28;
    }

    // Set the state of the interrupt to inactive, mark it as being in the List Register, and set the List Register index in the interrupt.
    interrupt.set_state(IrqState::IrqSInactive);
    interrupt.set_in_lr(true);
    interrupt.set_lr(lr_idx as u16);
    vgic.set_cpu_priv_curr_lrs(vcpu_id, lr_idx, int_id as u16);
    debug!("write lr: lr value {:#x}", lr);
    GICH.set_lr_by_idx(lr_idx, lr as u32);

    update_int_list(vgic, vcpu, interrupt);
    debug!("write lr: end");
}

fn route(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) {
    debug!("[route]");
    let cpu_id = current_cpu().cpu_id;
    if let IrqState::IrqSInactive = interrupt.state() {
        return;
    }

    if !interrupt.enabled() {
        return;
    }

    let int_targets = interrupt.targets();
    // not consider ipi in multi core
    debug!("route: int_targets {:#x}, irq: {}", int_targets, interrupt.id());
    add_lr(vgic, vcpu.clone(), interrupt.clone());
    /*if (int_targets & (1 << cpu_id)) != 0 {
        // debug!("vm{} route addr lr for int {}", vcpu.vm_id(), interrupt.id());
        add_lr(vgic, vcpu.clone(), interrupt.clone());
    }

    if !interrupt.in_lr() && (int_targets & !(1 << cpu_id)) != 0 {
        let vcpu_vm_id = vcpu.vm_id;

        let ipi_msg = IpiInitcMessage {
            event: InitcEvent::VgicdRoute,
            vm_id: vcpu_vm_id,
            int_id: interrupt.id(),
            val: 0,
        };
        vgic_int_yield_owner(vcpu, interrupt);
        ipi_intra_broadcast_msg(active_vm().unwrap(), IpiType::IpiTIntc, IpiInnerMsg::Initc(ipi_msg));
    }
    */
}

fn set_enable(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize, en: bool) {
    if int_id < GIC_SGIS_NUM {
        return;
    }
    match get_int(vgic, vcpu.clone(), int_id) {
        Some(interrupt) => {
            let interrupt_lock = interrupt.lock.lock();
            if interrupt.enabled() ^ en {
                interrupt.set_enabled(en);
                if !interrupt.enabled() {
                    remove_lr(vgic, vcpu.clone(), interrupt.clone());
                } else {
                    route(vgic, vcpu.clone(), interrupt.clone());
                }
                if interrupt.hw() {
                    GICD.lock().set_enable(interrupt.id() as usize, en);
                }
            }
            vgic_int_yield_owner(vcpu, interrupt.clone());
            /*
            if vgic_int_is_owner(vcpu.clone(), interrupt.clone()) {
                if interrupt.enabled() ^ en {
                    interrupt.set_enabled(en);
                    if !interrupt.enabled() {
                        remove_lr(vgic, vcpu.clone(), interrupt.clone());
                    } else {
                        route(vgic, vcpu.clone(), interrupt.clone());
                    }
                    if interrupt.hw() {
                        GICD.set_enable(interrupt.id() as usize, en);
                    }
                }
                vgic_int_yield_owner(vcpu, interrupt.clone());
            } else {
                let int_phys_id = interrupt.owner_phys_id().unwrap();
                let vcpu_vm_id = vcpu.vm_id();
                let ipi_msg = IpiInitcMessage {
                    event: InitcEvent::VgicdSetEn,
                    vm_id: vcpu_vm_id,
                    int_id: interrupt.id(),
                    val: en as u8,
                };
                if !ipi_send_msg(int_phys_id, IpiType::IpiTIntc, IpiInnerMsg::Initc(ipi_msg)) {
                    debug!(
                        "vgicd_set_enable: Failed to send ipi message, target {} type {}",
                        int_phys_id, 0
                    );
                }
            }
            */
            drop(interrupt_lock);
        }
        None => {
            debug!("vgicd_set_enable: interrupt {} is illegal", int_id);
            return;
        }
    }
}

fn get_enable(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize) -> bool {
    get_int(vgic, vcpu, int_id).unwrap().enabled()
}

fn set_pend(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize, pend: bool) {
    // TODO: sgi_get_pend ?
    if bit_extract(int_id, 0, 10) < GIC_SGIS_NUM {
        set_enable(vgic, vcpu, int_id, pend);
        return;
    }

    let interrupt_option = get_int(vgic, vcpu.clone(), bit_extract(int_id, 0, 10));

    if let Some(interrupt) = interrupt_option {
        let interrupt_lock = interrupt.lock.lock();
        remove_lr(vgic, vcpu.clone(), interrupt.clone());

        let state = interrupt.state().to_num();
        // set the state to right value
        if pend && ((state & 1) == 0) {
            interrupt.set_state(IrqState::num_to_state(state | 1));
        } else if !pend && (state & 1) != 0 {
            interrupt.set_state(IrqState::num_to_state(state & !1));
        }
        update_int_list(vgic, vcpu.clone(), interrupt.clone());

        let state = interrupt.state().to_num();
        if interrupt.hw() {
            let vgic_int_id = interrupt.id() as usize;
            GICD.lock().set_state(
                vgic_int_id,
                if state == 1 { 2 } else { state },
                current_cpu().cpu_id,
            )
        }
        route(vgic, vcpu.clone(), interrupt.clone());
        vgic_int_yield_owner(vcpu, interrupt.clone());
        drop(interrupt_lock);
        /*
        if vgic_int_is_owner(vcpu.clone(), interrupt.clone()) {
            remove_lr(vgic, vcpu.clone(), interrupt.clone());

            let state = interrupt.state().to_num();
            if pend && ((state & 1) == 0) {
                interrupt.set_state(IrqState::num_to_state(state | 1));
            } else if !pend && (state & 1) != 0 {
                interrupt.set_state(IrqState::num_to_state(state & !1));
            }
            update_int_list(vgic, vcpu.clone(), interrupt.clone());

            let state = interrupt.state().to_num();
            if interrupt.hw() {
                let vgic_int_id = interrupt.id() as usize;
                GICD.set_state(vgic_int_id, if state == 1 { 2 } else { state })
            }
            route(vgic, vcpu.clone(), interrupt.clone());
            vgic_int_yield_owner(vcpu, interrupt.clone());
            drop(interrupt_lock);
        } else {
            let vm_id = vcpu.vm_id();

            let m = IpiInitcMessage {
                event: InitcEvent::VgicdSetPend,
                vm_id,
                int_id: interrupt.id(),
                val: pend as u8,
            };
            match interrupt.owner() {
                Some(owner) => {
                    let phys_id = owner.phys_id();

                    drop(interrupt_lock);
                    if !ipi_send_msg(phys_id, IpiType::IpiTIntc, IpiInnerMsg::Initc(m)) {
                        debug!(
                            "vgicd_set_pend: Failed to send ipi message, target {} type {}",
                            phys_id, 0
                        );
                    }
                }
                None => {
                    panic!(
                        "set_pend: Core {} int {} has no owner",
                        current_cpu().id,
                        interrupt.id()
                    );
                }
            }
        }
        */
    }
}

fn set_active(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize, act: bool) {
    let interrupt_option = get_int(vgic, vcpu.clone(), bit_extract(int_id, 0, 10));
    if let Some(interrupt) = interrupt_option {
        let interrupt_lock = interrupt.lock.lock();
        remove_lr(vgic, vcpu.clone(), interrupt.clone());
        let state = interrupt.state().to_num();
        if act && ((state & 2) == 0) {
            interrupt.set_state(IrqState::num_to_state(state | 2));
        } else if !act && (state & 2) != 0 {
            interrupt.set_state(IrqState::num_to_state(state & !2));
        }
        update_int_list(vgic, vcpu.clone(), interrupt.clone());

        let state = interrupt.state().to_num();
        if interrupt.hw() {
            let vgic_int_id = interrupt.id() as usize;
            GICD.lock().set_state(
                vgic_int_id,
                if state == 1 { 2 } else { state },
                current_cpu().cpu_id,
            )
        }
        route(vgic, vcpu.clone(), interrupt.clone());
        vgic_int_yield_owner(vcpu, interrupt.clone());
        /*
        if vgic_int_is_owner(vcpu.clone(), interrupt.clone()) {
            remove_lr(vgic, vcpu.clone(), interrupt.clone());
            let state = interrupt.state().to_num();
            if act && ((state & 2) == 0) {
                interrupt.set_state(IrqState::num_to_state(state | 2));
            } else if !act && (state & 2) != 0 {
                interrupt.set_state(IrqState::num_to_state(state & !2));
            }
            update_int_list(vgic, vcpu.clone(), interrupt.clone());

            let state = interrupt.state().to_num();
            if interrupt.hw() {
                let vgic_int_id = interrupt.id() as usize;
                GICD.set_state(vgic_int_id, if state == 1 { 2 } else { state })
            }
            route(vgic, vcpu.clone(), interrupt.clone());
            vgic_int_yield_owner(vcpu, interrupt.clone());
        } else {
            let vm_id = vcpu.vm_id();

            let m = IpiInitcMessage {
                event: InitcEvent::VgicdSetPend,
                vm_id,
                int_id: interrupt.id(),
                val: act as u8,
            };
            let phys_id = interrupt.owner_phys_id().unwrap();
            if !ipi_send_msg(phys_id, IpiType::IpiTIntc, IpiInnerMsg::Initc(m)) {
                debug!(
                    "vgicd_set_active: Failed to send ipi message, target {} type {}",
                    phys_id, 0
                );
            }
        }
        */
        drop(interrupt_lock);
    }
}

fn set_icfgr(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize, cfg: u8) {
    let interrupt_option = get_int(vgic, vcpu.clone(), int_id);
    if let Some(interrupt) = interrupt_option {
        let interrupt_lock = interrupt.lock.lock();
        interrupt.set_cfg(cfg);
        if interrupt.hw() {
            GICD.lock().set_icfgr(interrupt.id() as usize, cfg);
        }
        vgic_int_yield_owner(vcpu, interrupt.clone());
        /*
        if vgic_int_is_owner(vcpu.clone(), interrupt.clone()) {
            interrupt.set_cfg(cfg);
            if interrupt.hw() {
                GICD.set_icfgr(interrupt.id() as usize, cfg);
            }
            vgic_int_yield_owner(vcpu, interrupt.clone());
        } else {
            let m = IpiInitcMessage {
                event: InitcEvent::VgicdSetCfg,
                vm_id: vcpu.vm_id(),
                int_id: interrupt.id(),
                val: cfg,
            };
            if !ipi_send_msg(
                interrupt.owner_phys_id().unwrap(),
                IpiType::IpiTIntc,
                IpiInnerMsg::Initc(m),
            ) {
                debug!(
                    "set_icfgr: Failed to send ipi message, target {} type {}",
                    interrupt.owner_phys_id().unwrap(),
                    0
                );
            }
        }
        */
        drop(interrupt_lock);
    } else {
        unimplemented!();
    }
}

fn get_icfgr(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize) -> u8 {
    let interrupt_option = get_int(vgic, vcpu, int_id);
    if let Some(interrupt) = interrupt_option {
        return interrupt.cfg();
    } else {
        unimplemented!();
    }
}

fn sgi_set_pend(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize, pend: bool) {
    // let begin = time_current_us();
    if bit_extract(int_id, 0, 10) > GIC_SGIS_NUM {
        return;
    }

    let interrupt_option = get_int(vgic, vcpu.clone(), bit_extract(int_id, 0, 10));
    let source = bit_extract(int_id, 10, 5);

    if let Some(interrupt) = interrupt_option {
        let interrupt_lock = interrupt.lock.lock();
        remove_lr(vgic, vcpu.clone(), interrupt.clone());
        let vcpu_id = vcpu.vcpu_id;

        let vgic_int_id = interrupt.id() as usize;
        let pendstate = vgic.cpu_priv_sgis_pend(vcpu_id, vgic_int_id);
        // let pendstate = cpu_priv[vcpu_id].sgis[vgic_int_id].pend;
        let new_pendstate = if pend {
            pendstate | (1 << source) as u8
        } else {
            pendstate & !(1 << source) as u8
        };
        if (pendstate ^ new_pendstate) != 0 {
            // cpu_priv[vcpu_id].sgis[vgic_int_id].pend = new_pendstate;
            vgic.set_cpu_priv_sgis_pend(vcpu_id, vgic_int_id, new_pendstate);
            let state = interrupt.state().to_num();
            if new_pendstate != 0 {
                interrupt.set_state(IrqState::num_to_state(state | 1));
            } else {
                interrupt.set_state(IrqState::num_to_state(state & !1));
            }

            update_int_list(vgic, vcpu.clone(), interrupt.clone());

            // debug!("state {}", interrupt.state().to_num());
            match interrupt.state() {
                IrqState::IrqSInactive => {
                    debug!("inactive");
                }
                _ => {
                    add_lr(vgic, vcpu, interrupt.clone());
                }
            }
        }
        drop(interrupt_lock);
    } else {
        debug!(
            "sgi_set_pend: interrupt {} is None",
            bit_extract(int_id, 0, 10)
        );
    }
    // let end = time_current_us();
    // debug!("sgi_set_pend[{}]", end - begin);
}

fn set_priority(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize, mut prio: u8) {
    let interrupt_option = get_int(vgic, vcpu.clone(), int_id);
    prio &= 0xf0; // gic-400 only allows 4 priority bits in non-secure state

    if let Some(interrupt) = interrupt_option {
        let interrupt_lock = interrupt.lock.lock();
        if interrupt.get_priority() != prio {
            remove_lr(vgic, vcpu.clone(), interrupt.clone());
            let prev_prio = interrupt.get_priority();
            interrupt.set_priority(prio);
            if prio <= prev_prio {
                route(vgic, vcpu.clone(), interrupt.clone());
            }
            if interrupt.hw() {
                GICD.lock().set_priority(interrupt.id() as usize, prio);
            }
        }
        vgic_int_yield_owner(vcpu, interrupt.clone());
        /*
        if vgic_int_is_owner(vcpu.clone(), interrupt.clone()) {
            if interrupt.get_priority() != prio {
                remove_lr(vgic, vcpu.clone(), interrupt.clone());
                let prev_prio = interrupt.get_priority();
                interrupt.set_priority(prio);
                if prio <= prev_prio {
                    route(vgic, vcpu.clone(), interrupt.clone());
                }
                if interrupt.hw() {
                    GICD.lock().set_priority(interrupt.id() as usize, prio);
                }
            }
            vgic_int_yield_owner(vcpu, interrupt.clone());
        } else {
            let vm_id = vcpu.vm_id;

            let m = IpiInitcMessage {
                event: InitcEvent::VgicdSetPrio,
                vm_id,
                int_id: interrupt.id(),
                val: prio,
            };
            if !ipi_send_msg(
                interrupt.owner_phys_id().unwrap(),
                IpiType::IpiTIntc,
                IpiInnerMsg::Initc(m),
            ) {
                debug!(
                    "set_priority: Failed to send ipi message, target {} type {}",
                    interrupt.owner_phys_id().unwrap(),
                    0
                );
            }
        }
        */
        drop(interrupt_lock);
    }
}

fn get_priority(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize) -> u8 {
    let interrupt_option = get_int(vgic, vcpu, int_id);
    return interrupt_option.unwrap().get_priority();
}

fn set_target(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize, target: u8) {
    let interrupt_option = get_int(vgic, vcpu.clone(), int_id);
    if let Some(interrupt) = interrupt_option {
        let interrupt_lock = interrupt.lock.lock();
        if interrupt.targets() != target {
            interrupt.set_targets(target);
            let mut ptrgt = 0;
            for cpuid in 0..8 {
                if bit_get(target as usize, cpuid) != 0 {
                    ptrgt = bit_set(ptrgt, cpuid)
                }
            }
            if interrupt.hw() {
                GICD.lock().set_target_cpu(interrupt.id() as usize, ptrgt as u8);
            }
            if vgic_get_state(interrupt.clone()) != 0 {
                route(vgic, vcpu.clone(), interrupt.clone());
            }
        }
        /*
        if vgic_int_is_owner(vcpu.clone(), interrupt.clone()) {
            if interrupt.targets() != target {
                interrupt.set_targets(target);
                let mut ptrgt = 0;
                for cpuid in 0..8 {
                    if bit_get(target as usize, cpuid) != 0 {
                        ptrgt = bit_set(ptrgt, Platform::cpuid_to_cpuif(cpuid))
                    }
                }
                if interrupt.hw() {
                    GICD.set_target(interrupt.id() as usize, ptrgt as u8);
                }
                if vgic_get_state(interrupt.clone()) != 0 {
                    route(vgic, vcpu.clone(), interrupt.clone());
                }
            }
            vgic_int_yield_owner(vcpu, interrupt.clone());
        } else {
            let vm_id = vcpu.vm_id();
            let m = IpiInitcMessage {
                event: InitcEvent::VgicdSetTrgt,
                vm_id,
                int_id: interrupt.id(),
                val: target,
            };
            if !ipi_send_msg(
                interrupt.owner_phys_id().unwrap(),
                IpiType::IpiTIntc,
                IpiInnerMsg::Initc(m),
            ) {
                debug!(
                    "set_target: Failed to send ipi message, target {} type {}",
                    interrupt.owner_phys_id().unwrap(),
                    0
                );
            }
        }
        */
        drop(interrupt_lock);
    }
}

fn get_target(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize) -> u8 {
    let interrupt_option = get_int(vgic, vcpu, int_id);
    return interrupt_option.unwrap().targets();
}

/// inject interrupt to vgic
pub fn vgic_inject(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, int_id: usize) {
    debug!("[vgic_inject] Core {} inject int {} to vm{}", current_cpu().cpu_id, int_id, vcpu.vm_id);
    let interrupt_option = get_int(vgic, vcpu.clone(), bit_extract(int_id, 0, 10));
    if let Some(interrupt) = interrupt_option {
        if interrupt.hw() {
            debug!("[vgic_inject] interrupt is hw");
            let interrupt_lock = interrupt.lock.lock();
            interrupt.set_owner(vcpu.clone());
            interrupt.set_state(IrqState::IrqSPend);
            update_int_list(vgic, vcpu.clone(), interrupt.clone());
            interrupt.set_in_lr(false);
            route(vgic, vcpu, interrupt.clone());
            drop(interrupt_lock);
        } else {
            set_pend(vgic, vcpu, int_id, true);
        }
    }
}

/// access emulated ctlr
pub fn emu_ctrl_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("[emu_ctrl_access] this is emu_ctrl_access");
    if emu_ctx.write {
        let prev_ctlr = vgic.vgicd_ctlr();
        let idx = emu_ctx.reg;
        vgic.set_vgicd_ctlr(current_cpu().get_gpr(idx) as u32 & 0x1);
        // only one cpu for a vm, do not need ipi broadcast?
        /*
        if prev_ctlr ^ vgic.vgicd_ctlr() != 0 {
            let enable = vgic.vgicd_ctlr() != 0;
            let hcr = GICH.get_hcr();
            if enable {
                GICH.set_hcr(hcr | 1);
            } else {
                GICH.set_hcr(hcr & !1);
            }

            let m = IpiInitcMessage {
                event: InitcEvent::VgicdGichEn,
                vm_id: active_vm_id(),
                int_id: 0,
                val: enable as u8,
            };
            ipi_intra_broadcast_msg(
                active_vm().unwrap(),
                IpiType::IpiTIntc,
                IpiInnerMsg::Initc(m),
            );
        }
        */
    } else {
        let idx = emu_ctx.reg;
        let val = vgic.vgicd_ctlr() as usize;
        current_cpu().set_gpr(idx, val);
    }
}

/// access emulated typer
pub fn emu_typer_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("[emu_typer_access] this is emu_typer_access");
    if !emu_ctx.write {
        let idx = emu_ctx.reg;
        let val = vgic.vgicd_typer() as usize;
        current_cpu().set_gpr(idx, val);
    } else {
        debug!("emu_typer_access: can't write to RO reg");
    }
}

/// access emulated iidr
pub fn emu_iidr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("[emu_iidr_access] this is emu_iidr_access");
    if !emu_ctx.write {
        let idx = emu_ctx.reg;
        let val = vgic.vgicd_iidr() as usize;
        current_cpu().set_gpr(idx, val);
    } else {
        debug!("[emu_iidr_access]: can't write to RO reg");
    }
}

/// access emulated gicd enable group
pub fn emu_enabler_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext, set: bool) {
    debug!("[emu_enabler_access] this is emu_enabler_access emu_ctx: {:?}", emu_ctx);
    // the offset of the required GICD_IxENABLER<n> is (reg base + (4*n))
    // reg_idx is <n>
    let reg_idx = (emu_ctx.address & 0b111_1111) / 4;
    let idx = emu_ctx.reg;
    let mut val = if emu_ctx.write {
        current_cpu().get_gpr(idx)
    } else {
        0
    };
    debug!("[emu_enabler_access] this is write val:{:#x}", val);
    // caculate the first interrupt in the <n>th register
    let first_int = reg_idx * 32;
    let vm_id = active_vm().vm_id;
    let vm = active_vm();
    let mut vm_has_interrupt_flag = false;

    for i in 0..32 {
        if vm.has_interrupt(first_int + i) || vm.emu_has_interrupt(first_int + i) {
            debug!("[emu_enabler_access] this is vm has interrupt: {}", first_int+i);
            vm_has_interrupt_flag = true;
            break;
        }
    }
    if first_int >= 16 && !vm_has_interrupt_flag {
        debug!(
            "[emu_isenabler_access]: vm[{}] does not have interrupt {}",
            vm_id, first_int
        );
        return;
    }

    if emu_ctx.write {
        for i in 0..32 {
            if bit_get(val, i) != 0 {
                set_enable(vgic, current_cpu().get_active_vcpu().unwrap().clone(), first_int + i, set);
                debug!("[emu_enabler_access] set interrupt enable: {:#x} first_int: {:#x}", first_int + i, first_int);
            }
        }
    } else {
        for i in 0..32 {
            if get_enable(vgic, current_cpu().get_active_vcpu().unwrap().clone(), first_int + i) {
                val |= 1 << i;
            }
        }
        let idx = emu_ctx.reg;
        current_cpu().set_gpr(idx, val);
    }
}

/// access emulated gicd isenable
pub fn emu_isenabler_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("[emu_isenabler_access] this is emu_isenabler_access");
    emu_enabler_access(vgic, emu_ctx, true);
}

/// access emulated gicd icenabler
pub fn emu_icenabler_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("[emu_icenabler_access] this is emu_icenabler_access");
    emu_enabler_access(vgic, emu_ctx, false);
}

/// access emulated gicd pend group
pub fn emu_pendr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext, set: bool) {
    debug!("[emu_pendr_access] this is emu_pendr_access");
    // the offset of the required GICD_IxPENDR<n> is (reg base + (4*n))
    // reg_idx is <n>
    let reg_idx = (emu_ctx.address & 0b1111111) / 4;
    let idx = emu_ctx.reg;
    let mut val = if emu_ctx.write {
        current_cpu().get_gpr(idx)
    } else {
        0
    };

    // caculate the first interrupt in the <n>th register
    let first_int = reg_idx * 32;
    let vm_id = active_vm().vm_id;
    let vm = active_vm();
    let mut vm_has_interrupt_flag = false;

    for i in 0..emu_ctx.width {
        if vm.has_interrupt(first_int + i) || vm.emu_has_interrupt(first_int + i) {
            vm_has_interrupt_flag = true;
            break;
        }
    }
    if first_int >= 16 && !vm_has_interrupt_flag {
        debug!(
            "emu_pendr_access: vm[{}] does not have interrupt {}",
            vm_id, first_int
        );
        return;
    }

    if emu_ctx.write {
        for i in 0..32 {
            if bit_get(val, i) != 0 {
                set_pend(vgic, current_cpu().get_active_vcpu().unwrap().clone(), first_int + i, set);
            }
        }
    } else {
        for i in 0..32 {
            match get_int(vgic, current_cpu().get_active_vcpu().unwrap().clone(), first_int + i) {
                Some(interrupt) => {
                    if vgic_get_state(interrupt.clone()) & 1 != 0 {
                        val |= 1 << i;
                    }
                }
                None => {}
            }
        }
        let idx = emu_ctx.reg;
        current_cpu().set_gpr(idx, val);
    }
}

/// access emulated gicd ispendr
pub fn emu_ispendr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("[emu_ispendr_access] this is emu_ispendr_access");
    emu_pendr_access(vgic, emu_ctx, true);
}

/// access emulated gicd icpendr
pub fn emu_icpendr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("[emu_icpendr_access] this is emu_icpendr_access");
    emu_pendr_access(vgic, emu_ctx, false);
}

/// access emulated gicd active group
pub fn emu_activer_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext, set: bool) {
    debug!("[emu_activer_access] this is emu_activer_access");
    // the offset of the required GICD_IxACTIVER<n> is (reg base + (4*n))
    // reg_idx is <n>
    let reg_idx = (emu_ctx.address & 0b111_1111) / 4;
    let idx = emu_ctx.reg;
    let mut val = if emu_ctx.write {
        current_cpu().get_gpr(idx)
    } else {
        0
    };

    // caculate the first interrupt in the <n>th register
    let first_int = reg_idx * 32;
    let vm_id = active_vm().vm_id;
    let vm = active_vm();
    let mut vm_has_interrupt_flag = false;

    for i in 0..32 {
        if vm.has_interrupt(first_int + i) || vm.emu_has_interrupt(first_int + i) {
            vm_has_interrupt_flag = true;
            break;
        }
    }
    if first_int >= 16 && !vm_has_interrupt_flag {
        debug!(
            "emu_activer_access: vm[{}] does not have interrupt {}",
            vm_id, first_int
        );
        return;
    }

    if emu_ctx.write {
        for i in 0..32 {
            if bit_get(val, i) != 0 {
                set_active(vgic, current_cpu().get_active_vcpu().unwrap().clone(), first_int + i, set);
            }
        }
    } else {
        for i in 0..32 {
            match get_int(vgic, current_cpu().get_active_vcpu().unwrap().clone(), first_int + i) {
                Some(interrupt) => {
                    if vgic_get_state(interrupt.clone()) & 2 != 0 {
                        val |= 1 << i;
                    }
                }
                None => {}
            }
        }
        let idx = emu_ctx.reg;
        current_cpu().set_gpr(idx, val);
    }
}

/// access emulated gicd isactiver
pub fn emu_isactiver_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("[emu_isactiver_access] this is emu_isactiver_access");
    emu_activer_access(vgic, emu_ctx, true);
}

/// access emulated gicd icactiver
pub fn emu_icactiver_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("[emu_icactiver_access] this is emu_icactiver_access");
    emu_activer_access(vgic, emu_ctx, false);
}

/// access emulated gicd icfgr
pub fn emu_icfgr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("[emu_icfgr_access] this is emu_icfgr_access");
    let first_int = (32 / GIC_CONFIG_BITS) * bit_extract(emu_ctx.address, 0, 9) / 4;
    let vm_id = active_vm().vm_id;
    let vm = active_vm();
    let mut vm_has_interrupt_flag = false;

    if emu_ctx.write {
        for i in 0..emu_ctx.width * 8 {
            if vm.has_interrupt(first_int + i) || vm.emu_has_interrupt(first_int + i) {
                vm_has_interrupt_flag = true;
                break;
            }
        }
        if first_int >= 16 && !vm_has_interrupt_flag {
            debug!(
                "[emu_icfgr_access]: vm[{}] does not have interrupt {}",
                vm_id, first_int
            );
            return;
        }
    }

    if emu_ctx.write {
        let idx = emu_ctx.reg;
        let cfg = current_cpu().get_gpr(idx);
        let mut irq = first_int;
        let mut bit = 0;
        while bit < emu_ctx.width * 8 {
            set_icfgr(vgic, 
                current_cpu().get_active_vcpu().unwrap().clone(),
                irq,
                bit_extract(cfg as usize, bit, 2) as u8,
            );
            bit += 2;
            irq += 1;
        }
    } else {
        let mut cfg = 0;
        let mut irq = first_int;
        let mut bit = 0;
        while bit < emu_ctx.width * 8 {
            cfg |= (get_icfgr(vgic, current_cpu().get_active_vcpu().unwrap().clone(), irq) as usize) << bit;
            bit += 2;
            irq += 1;
        }
        let idx = emu_ctx.reg;
        let val = cfg;
        current_cpu().set_gpr(idx, val);
    }
}

/// access emulated gicd sgi related registers
pub fn emu_sgiregs_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("this is emu_sgiregs_access");
    let idx = emu_ctx.reg;
    let val = if emu_ctx.write {
        current_cpu().get_gpr(idx)
    } else {
        0
    };
    let vm = active_vm();

    // if the address is sgir (offset 0x0f00)
    if bit_extract(emu_ctx.address, 0, 12) == bit_extract(usize::from(axhal::GICD_BASE + 0x0f00) + 0x0f00, 0, 12) {
        if emu_ctx.write {
            // TargetListFilter, bits [25:24] Determines how the Distributor processes the requested SGI.
            let sgir_target_list_filter = bit_extract(val, 24, 2);
            let mut trgtlist = 0;
            match sgir_target_list_filter {
                // 0b00 Forward the interrupt to the CPU interfaces specified by GICD_SGIR.CPUTargetList[23:16].
                0 => {
                    trgtlist = vgic_target_translate(vm, bit_extract(val, 16, 8) as u32, true)
                        as usize;
                }
                // 0b01 Forward the interrupt to all CPU interfaces except that of the PE that requested the interrupt.
                1 => {
                    // todo: implement multi cpu for one vm
                    // trgtlist = active_vm_ncpu() & !(1 << current_cpu().id);
                }
                // 0b10 Forward the interrupt only to the CPU interface of the PE that requested the interrupt.
                2 => {
                    trgtlist = 1 << current_cpu().cpu_id;
                }
                // 0b11 Reserved.
                3 => {
                    return;
                }
                _ => {}
            }
            // GICv2 only support 8 pe. doto sgi between multi core
            /*
            for i in 0..8 {
                if trgtlist & (1 << i) != 0 {
                    let m = IpiInitcMessage {
                        event: InitcEvent::VgicdSetPend,
                        vm_id: active_vm_id(),
                        int_id: (bit_extract(val, 0, 8) | (active_vcpu_id() << 10)) as u16,
                        val: true as u8,
                    };
                    if !ipi_send_msg(i, IpiType::IpiTIntc, IpiInnerMsg::Initc(m)) {
                        debug!(
                            "emu_sgiregs_access: Failed to send ipi message, target {} type {}",
                            i, 0
                        );
                    }
                }
            }
            */
        }
    } else {
        // TODO: CPENDSGIR and SPENDSGIR access
        debug!("unimplemented: CPENDSGIR and SPENDSGIR access");
    }
}

/// access emulated gicd ipriorityr
pub fn emu_ipriorityr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("this is emu_ipriorityr_access");
    let idx = emu_ctx.reg;
    let mut val = if emu_ctx.write {
        current_cpu().get_gpr(idx)
    } else {
        0
    };
    let first_int = (8 / GIC_PRIO_BITS) * bit_extract(emu_ctx.address, 0, 9);
    let vm_id = active_vm().vm_id;
    let vm = active_vm();
    let mut vm_has_interrupt_flag = false;

    if emu_ctx.write {
        for i in 0..emu_ctx.width {
            if vm.has_interrupt(first_int + i) || vm.emu_has_interrupt(first_int + i) {
                vm_has_interrupt_flag = true;
                break;
            }
        }
        if first_int >= 16 && !vm_has_interrupt_flag {
            debug!(
                "emu_ipriorityr_access: vm[{}] does not have interrupt {}",
                vm_id, first_int
            );
            return;
        }
    }

    if emu_ctx.write {
        for i in 0..emu_ctx.width {
            set_priority(vgic, 
                current_cpu().get_active_vcpu().unwrap().clone(),
                first_int + i,
                bit_extract(val, GIC_PRIO_BITS * i, GIC_PRIO_BITS) as u8,
            );
        }
    } else {
        for i in 0..emu_ctx.width {
            val |= (get_priority(vgic, current_cpu().get_active_vcpu().unwrap().clone(), first_int + i)
                as usize)
                << (GIC_PRIO_BITS * i);
        }
        let idx = emu_ctx.reg;
        current_cpu().set_gpr(idx, val);
    }
}

/// access emulated gicd itargetr
pub fn emu_itargetr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    debug!("[emu_itargetr_access] this is emu_itargetr_access");
    let idx = emu_ctx.reg;
    let mut val = if emu_ctx.write {
        current_cpu().get_gpr(idx)
    } else {
        0
    };
    let first_int = (8 / GIC_TARGET_BITS) * bit_extract(emu_ctx.address, 0, 9);

    if emu_ctx.write {
        val = vgic_target_translate(active_vm(), val as u32, true) as usize;
        for i in 0..emu_ctx.width {
            set_target(vgic, 
                current_cpu().get_active_vcpu().unwrap().clone(),
                first_int + i,
                bit_extract(val, GIC_TARGET_BITS * i, GIC_TARGET_BITS) as u8,
            );
        }
    } else {
        // debug!("read, first_int {}, width {}", first_int, emu_ctx.width);
        for i in 0..emu_ctx.width {
            // debug!("{}", get_target(vgic, active_vcpu().unwrap(), first_int + i));
            val |= (get_target(vgic, current_cpu().get_active_vcpu().unwrap().clone(), first_int + i) as usize)
                << (GIC_TARGET_BITS * i);
        }
        debug!("[emu_itargetr_access] after read val {}", val);
        val = vgic_target_translate(active_vm(), val as u32, false) as usize;
        let idx = emu_ctx.reg;
        current_cpu().set_gpr(idx, val);
    }
    debug!("[emu_itargetr_access] in the end of emu_itargetr_access");
}

// End Of Interrupt maintenance interrupt asserted.
pub fn handle_trapped_eoir(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>) {
    debug!("this is handle_trapped_eoir");
    let gic_lrs = gic_lrs();
    // find the first 1 in eisr0 and eisr1
    let mut lr_idx_option = bitmap_find_nth(
        GICH.get_eisr_by_idx(0) as usize | ((GICH.get_eisr_by_idx(1) as usize) << 32),
        0,
        gic_lrs,
        1,
        true,
    );
    // clear eoi lr circularly
    while lr_idx_option.is_some() {
        // clear corresponding lr
        let lr_idx = lr_idx_option.unwrap();
        let lr_val = GICH.get_lr_by_idx(lr_idx) as usize;
        GICH.set_lr_by_idx(lr_idx, 0);

        // clear interrupt state, set it not in lr
        match get_int(vgic, vcpu.clone(), bit_extract(lr_val, 0, 10)) {
            Some(interrupt) => {
                let interrupt_lock = interrupt.lock.lock();
                interrupt.set_in_lr(false);
                if (interrupt.id() as usize) < GIC_SGIS_NUM {
                    add_lr(vgic, vcpu.clone(), interrupt.clone());
                } else {
                    vgic_int_yield_owner(vcpu.clone(), interrupt.clone());
                }
                drop(interrupt_lock);
            }
            None => {
                unimplemented!();
            }
        }
        lr_idx_option = bitmap_find_nth(
            GICH.get_eisr_by_idx(0) as usize | ((GICH.get_eisr_by_idx(1) as usize) << 32),
            0,
            gic_lrs,
            1,
            true,
        );
    }
}

// No Pending maintenance interrupt asserted. (no List register is in the pending state.)
pub fn refill_lrs(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>) {
    debug!("this is refill_lrs");
    let gic_lrs = gic_lrs();
    let mut has_pending = false;

    for i in 0..gic_lrs {
        let lr = GICH.get_lr_by_idx(i) as usize;
        // [29:28] state. 0b1: pending
        if bit_extract(lr, 28, 2) & 1 != 0 {
            has_pending = true;
        }
    }

    // Find the index of the first empty LR.
    let mut lr_idx_option = bitmap_find_nth(
        GICH.get_elrsr_by_idx(0) as usize | ((GICH.get_elrsr_by_idx(1) as usize) << 32),
        0,
        gic_lrs,
        1,
        true,
    );

    // refill empty LR  until there is no more empty LR.
    while lr_idx_option.is_some() {
        let mut interrupt_opt: Option<VgicInt<HyperCraftHalImpl, GuestPageTable>> = None;
        let mut prev_pend = false;
        // Get the first active and pending interrupts.
        let active_head = int_list_head(vgic, vcpu.clone(), false);
        let pend_head = int_list_head(vgic, vcpu.clone(), true);
        // firstly add first active interrupt to lr if it is not in lr, otherwise add first pending interrupt to lr if it is not in lr
        if has_pending {
            match active_head {
                Some(active_int) => {
                    if !active_int.in_lr() {
                        interrupt_opt = Some(active_int.clone());
                    }
                }
                None => {}
            }
        }
        if interrupt_opt.is_none() {
            if let Some(pend_int) = pend_head {
                if !pend_int.in_lr() {
                    interrupt_opt = Some(pend_int.clone());
                    prev_pend = true;
                }
            }
        }

        // If an interrupt has been selected...
        match interrupt_opt {
            Some(interrupt) => {
                vgic_int_is_owner(vcpu.clone(), interrupt.clone());
                write_lr(vgic, vcpu.clone(), interrupt.clone(), lr_idx_option.unwrap());
                has_pending = has_pending || prev_pend;
            }
            None => {
                // debug!("no int to refill");
                // If no interrupt has been selected, disable the LR refill maintenance.
                let hcr = GICH.get_hcr();
                GICH.set_hcr(hcr & !(1 << 3));
                break;
            }
        }

        lr_idx_option = bitmap_find_nth(
            GICH.get_elrsr_by_idx(0) as usize | ((GICH.get_elrsr_by_idx(1) as usize) << 32),
            0,
            gic_lrs,
            1,
            true,
        );
    }
    // debug!("end refill lrs");
}

// List Register Entry Not Present maintenance interrupt asserted.
// Generic Interrupt Controller (GIC) has attempted to access an interrupt that is not present in any of the List Registers (LRs).
pub fn eoir_highest_spilled_active(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>) {
    debug!("this is eoir_highest_spilled_active");
    // get pending interrupt
    let interrupt = int_list_head(vgic, vcpu.clone(), false);
    match interrupt {
        Some(int) => {
            int.lock.lock();
            // if interrupt does not have an owner, set it to current vcpu
            vgic_int_is_owner(vcpu.clone(), int.clone());

            let state = int.state().to_num();
            // if state is active, set it to inactive
            int.set_state(IrqState::num_to_state(state & !2));
            update_int_list(vgic, vcpu.clone(), int.clone());

            if vgic_int_is_hw(int.clone()) {
                GICD.lock().set_active(int.id() as usize, false);
            } else {
                if int.state().to_num() & 1 != 0 {
                    add_lr(vgic, vcpu, int);
                }
            }
        }
        None => {}
    }
}

fn vgic_target_translate(vm:&mut VM<HyperCraftHalImpl, GuestPageTable>, target: u32, v2p: bool) -> u32 {
    let from = target.to_le_bytes();
    let mut result = 0;
    let converted_values = from.map(|x| {
        if v2p {
            vm.vcpu_to_pcpu_mask(x as usize, 8) as u32
        } else {
            vm.pcpu_to_vcpu_mask(x as usize, 8) as u32
        }
    });
    // debug!("print converted_values{:?}", converted_values.len());
    for (idx, val) in converted_values
        .iter()
        .enumerate()
    {
        // debug!("idx {} val{}", idx, val);
        result |= (*val as u32) << (8 * idx);
        if idx >= 4 {
            panic!("illegal idx, from len {}", from.len());
        }
    }
    result
}

fn vgic_owns(
    vcpu: VCpu<HyperCraftHalImpl>,
    interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>,
) -> bool {
    if gic_is_priv(interrupt.id() as usize) {
        return true;
    }

    let vcpu_id = vcpu.vcpu_id;
    let pcpu_id = vcpu.pcpu_id;
    match interrupt.owner() {
        Some(owner) => {
            let owner_vcpu_id = owner.vcpu_id;
            let owner_pcpu_id = owner.pcpu_id;
            return owner_vcpu_id == vcpu_id && owner_pcpu_id == pcpu_id;
        }
        None => return false,
    }
}

fn vgic_get_state(interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) -> usize {
    let mut state = interrupt.state().to_num();

    if interrupt.in_lr() && interrupt.owner_phys_id().unwrap() == current_cpu().cpu_id {
        let lr_option = gich_get_lr(interrupt.clone());
        if let Some(lr_val) = lr_option {
            state = lr_val as usize;
        }
    }

    if interrupt.id() as usize >= GIC_SGIS_NUM {
        return state;
    }
    if interrupt.owner().is_none() {
        return state;
    }

    let vm = get_vm(interrupt.owner_vm_id().unwrap()).unwrap();
    let vgic = vm.vgic();
    let vcpu_id = interrupt.owner_id().unwrap();

    if vgic.cpu_priv_sgis_pend(vcpu_id, interrupt.id() as usize) != 0 {
        state |= 1;
    }

    state
}

fn vgic_int_yield_owner(
    vcpu: VCpu<HyperCraftHalImpl>,
    interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>,
) {
    // the vcpu is not the interrupt owner
    if !vgic_owns(vcpu, interrupt.clone()) {
        return;
    }
    // the interrupt is cpu private int or it has already been in lr
    if usize::from(interrupt.id()) < GIC_PRIVATE_INT_NUM || interrupt.in_lr() {
        return;
    }
    // if this interrupt is not active, clear its owner.
    if vgic_get_state(interrupt.clone()) & 2 == 0 {
        interrupt.clear_owner();
    }
}

fn gich_get_lr(interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) -> Option<u32> {
    let cpu_id = current_cpu().cpu_id;
    let phys_id = interrupt.owner_phys_id().unwrap();

    if !interrupt.in_lr() || phys_id != cpu_id {
        return None;
    }

    let lr_val = GICH.get_lr_by_idx(interrupt.lr() as usize);
    // interrupt is in lr and is pending or active
    if (lr_val & 0b11_1111_1111 == interrupt.id() as u32) && (lr_val >> 28 & 0b11 != 0) {
        return Some(lr_val as u32);
    }
    return None;
}

fn vgic_int_is_owner(
    vcpu: VCpu<HyperCraftHalImpl>,
    interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>,
) -> bool {
    // if interrupt.owner().is_none() {
    //     interrupt.set_owner(vcpu.clone());
    //     return true;
    // }
    let vcpu_id = vcpu.vcpu_id;
    let vcpu_vm_id = vcpu.vm_id;

    match interrupt.owner() {
        Some(owner) => {
            let owner_vcpu_id = owner.vcpu_id();
            let owner_vm_id = owner.vm_id;

            return owner_vm_id == vcpu_vm_id && owner_vcpu_id == vcpu_id;
        }
        None => {
            interrupt.set_owner(vcpu);
            return true;
        }
    }

    // let owner_vcpu_id = interrupt.owner_id().unwrap();
    // let owner_vm_id = interrupt.owner_vm_id().unwrap();

    return false;
}

pub fn vgic_set_hw_int(vm:&mut VM<HyperCraftHalImpl, GuestPageTable>, int_id: usize) {
    if int_id < GIC_SGIS_NUM {
        return;
    }
    /*
        if !vm.has_vgic() {
            return;
        }
    */
    let vgic = vm.vgic();

    if int_id < GIC_PRIVATE_INT_NUM {
        for i in 0..vm.vcpu_num() {
            let interrupt_option = get_int(&vgic, vm.vcpu(i).unwrap().clone(), int_id);
            match interrupt_option {
                Some(interrupt) => {
                    let interrupt_lock = interrupt.lock.lock();
                    interrupt.set_hw(true);
                    drop(interrupt_lock);
                }
                None => {}
            }
        }
    } else {
        let interrupt_option = get_int(&vgic, vm.vcpu(0).unwrap().clone(), int_id);
        match interrupt_option {
            Some(interrupt) => {
                let interrupt_lock = interrupt.lock.lock();
                interrupt.set_hw(true);
                drop(interrupt_lock);
            }
            None => {}
        }
    }
}

fn vgic_int_is_hw(interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) -> bool {
    interrupt.id() as usize >= GIC_SGIS_NUM && interrupt.hw()
}
