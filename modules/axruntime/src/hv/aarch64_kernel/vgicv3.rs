

use hypercraft::arch::vgic::{Vgic, VgicInt};
use hypercraft::{IrqState, VCpu, VM};
use crate::{GuestPageTable, HyperCraftHalImpl};

use arm_gicv3::{GIC_PRIVINT_NUM, GIC_INTS_MAX, GICH_LR_STATE_OFF, GICH_LR_STATE_LEN, GICH_HCR_NPIE_BIT,
                GICH_LR_VID_OFF, GICH_LR_VID_LEN, 
                // add_lr
                GICH_LR_PRIO_MSK, GICH_LR_PRIO_OFF, GICH_LR_STATE_MSK, GICH_LR_VID_MASK, GICH_LR_STATE_ACT, 
                GICH_LR_STATE_PND, 
                // vgic_int_is_hw
                GIC_SGIS_NUM,
                // write_lr
                GICH_LR_HW_BIT, GICH_LR_PID_OFF, GICH_LR_PID_MSK, GICH_LR_EOI_BIT, GICH_LR_GRP_BIT,   
            };

use arm_gicv3::GICH;
use arm_gicv3::platform::PLAT_DESC;

use arm_gicv3::gic_is_priv;
use hypercraft::arch::utils::bit_extract;
use hypercraft::arch::utils::bit_get;

use axhal::cpu::this_cpu_id;

use axhal::gicv3::gic_lrs;

// remove interrupt from vgic list
fn remove_int_list(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>, is_pend: bool) {
    let mut cpu_priv = vgic.cpu_priv.lock();
    let vcpu_id = vcpu.vcpu_id;
    let int_id = interrupt.id();
    if interrupt.in_lr() {
        if is_pend {
            if !interrupt.in_pend() {
                return;
            }
            for i in 0..cpu_priv[vcpu_id].pend_list.len() {
                if cpu_priv[vcpu_id].pend_list[i].id() == int_id {
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
}


fn add_int_list(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>, is_pend: bool) {
    let mut cpu_priv = vgic.cpu_priv.lock();
    let vcpu_id = vcpu.vcpu_id;
    if !interrupt.in_lr() {
        if is_pend {
            interrupt.set_in_pend_state(true);
            cpu_priv[vcpu_id].pend_list.push_back(interrupt);
        } else {
            interrupt.set_in_act_state(true);
            cpu_priv[vcpu_id].act_list.push_back(interrupt);
        }
    }
}

// update vgic int list according to the coming interrupt state
fn update_int_list(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) {
    let state = interrupt.state().to_num();

    if state & IrqState::IrqSPend.to_num() != 0 && !interrupt.in_pend() {
        add_int_list(vgic, vcpu.clone(), interrupt.clone(), true);
    } else if state & IrqState::IrqSPend.to_num() == 0 {
        remove_int_list(vgic, vcpu.clone(), interrupt.clone(), true);
    }

    if state & IrqState::IrqSActive.to_num() != 0 && !interrupt.in_act() {
        add_int_list(vgic, vcpu.clone(), interrupt.clone(), false);
    } else if state & IrqState::IrqSActive.to_num() == 0 {
        remove_int_list(vgic, vcpu.clone(), interrupt.clone(), false);
    }
}


fn int_list_head(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>,
        is_pend: bool) -> Option<VgicInt<HyperCraftHalImpl, GuestPageTable>> {
    let cpu_priv = vgic.cpu_priv.lock();
    let vcpu_id = vcpu.vcpu_id;
    if is_pend {
        if cpu_priv[vcpu_id].pend_list.is_empty() {
            None
        } else {
            Some(cpu_priv[vcpu_id].pend_list[0].clone())
        }
    } else if cpu_priv[vcpu_id].act_list.is_empty() {
        None
    } else {
        Some(cpu_priv[vcpu_id].act_list[0].clone())
    }
}


fn get_int(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>,
        int_id: usize) -> Option<VgicInt<HyperCraftHalImpl, GuestPageTable>> {
    if int_id < GIC_PRIVINT_NUM {
        let vcpu_id = vcpu.vcpu_id;
        return Some(vgic.cpu_priv_interrupt(vcpu_id, int_id));
    } else if (GIC_PRIVINT_NUM..GIC_INTS_MAX).contains(&int_id) {
        return Some(vgic.vgicd_interrupt(int_id - GIC_PRIVINT_NUM));
    }
    None
}


// remove_lr uses
fn vgic_owns(vcpu: VCpu<HyperCraftHalImpl>, interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) 
        -> bool {
    if gic_is_priv(interrupt.id() as usize) {
        return true;
    }

    let vcpu_id = vcpu.vcpu_id;
    let pcpu_id = vcpu.pcpu_id;
    match interrupt.owner() {
        Some(owner) => {
            let owner_vcpu_id = owner.vcpu_id;
            let owner_pcpu_id = owner.pcpu_id;
            owner_vcpu_id == vcpu_id && owner_pcpu_id == pcpu_id
        }
        None => false,
    }
}

// remove_lr uses
fn gich_get_lr(interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) 
        -> Option<usize> {
    let cpu_id = this_cpu_id();
    let phys_id = interrupt.owner_phys_id().unwrap();

    if !interrupt.in_lr() || phys_id != cpu_id {
        return None;
    }

    let lr_val = GICH.lr(interrupt.lr() as usize);
    if (bit_extract(lr_val, GICH_LR_VID_OFF, GICH_LR_VID_LEN) == interrupt.id() as usize)
        && (bit_extract(lr_val, GICH_LR_STATE_OFF, GICH_LR_STATE_LEN) != IrqState::IrqSInactive.to_num())
    {
        return Some(lr_val);
    }
    None
}

fn vgic_get_state(interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) 
        -> usize {
    let mut state = interrupt.state().to_num();

    if interrupt.in_lr() && interrupt.owner_phys_id().unwrap() == this_cpu_id() {
        let lr_option = gich_get_lr(interrupt.clone());
        if let Some(lr_val) = lr_option {
            state = bit_extract(lr_val, GICH_LR_STATE_OFF, GICH_LR_STATE_LEN);
        }
    }

    state
}

// add_lr uses
fn vgic_int_yield_owner(vcpu: VCpu<HyperCraftHalImpl>, interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) {
    if !vgic_owns(vcpu, interrupt.clone())
        || interrupt.in_lr()
        || (vgic_get_state(interrupt.clone()) & IrqState::IrqSActive.to_num() != 0)
    {
        return;
    }

    interrupt.clear_owner();
}


#[inline(always)] fn vgic_int_is_hw(interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) -> bool {
    interrupt.id() as usize >= GIC_SGIS_NUM && interrupt.hw()
}

fn remove_lr(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) -> bool {
    if !vgic_owns(vcpu.clone(), interrupt.clone()) {
        return false;
    }
    let int_lr = interrupt.lr();

    if !interrupt.in_lr() {
        return false;
    }

    let mut lr_val = 0;
    if let Some(lr) = gich_get_lr(interrupt.clone()) {
        GICH.set_lr(int_lr as usize, 0);
        lr_val = lr;
    }

    interrupt.set_in_lr(false);

    let lr_state = bit_extract(lr_val, GICH_LR_STATE_OFF, GICH_LR_STATE_LEN);
    if lr_state != IrqState::IrqSInactive.to_num() {
        interrupt.set_state(IrqState::num_to_state(lr_state));

        update_int_list(vgic, vcpu.clone(), interrupt.clone());

        if (interrupt.state().to_num() & IrqState::IrqSPend.to_num() != 0) && interrupt.enabled() {
            let hcr = GICH.hcr();
            GICH.set_hcr(hcr | GICH_HCR_NPIE_BIT);
        }
        return true;
    }
    false
}

fn add_lr(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) -> bool {
    if !interrupt.enabled() || interrupt.in_lr() {
        return false;
    }

    let gic_lrs = gic_lrs();
    let mut lr_ind = None;

    let elrsr = GICH.elrsr();
    //look for empty lr for using whit ICH_ELRSR_EL2
    for i in 0..gic_lrs {
        if bit_get(elrsr, i % 32) != 0 {
            lr_ind = Some(i);
            break;
        }
    }

    // if there is no empty then, replace one
    if lr_ind.is_none() {
        let mut pend_found = 0;
        // let mut act_found = 0;
        let mut min_prio_act = interrupt.get_priority() as usize;
        let mut min_prio_pend = interrupt.get_priority() as usize;
        let mut min_id_act = interrupt.id() as usize;
        let mut min_id_pend = interrupt.id() as usize;
        let mut act_ind = None;
        let mut pend_ind = None;

        for i in 0..gic_lrs {
            let lr = GICH.lr(i);
            let lr_prio = (lr & GICH_LR_PRIO_MSK) >> GICH_LR_PRIO_OFF;
            let lr_state = lr & GICH_LR_STATE_MSK;
            let lr_id = (lr & GICH_LR_VID_MASK) >> GICH_LR_VID_OFF;

            // look for min_prio act/pend lr (the value bigger then prio smaller)
            if lr_state & GICH_LR_STATE_ACT != 0 {
                if lr_prio > min_prio_act || (lr_prio == min_prio_act && lr_id > min_id_act) {
                    min_id_act = lr_id;
                    min_prio_act = lr_prio;
                    act_ind = Some(i);
                }
                // act_found += 1;
            } else if lr_state & GICH_LR_STATE_PND != 0 {
                if lr_prio > min_prio_pend || (lr_prio == min_prio_pend && lr_id > min_id_pend) {
                    min_id_pend = lr_id;
                    min_prio_pend = lr_prio;
                    pend_ind = Some(i);
                }
                pend_found += 1;
            }
        }

        // replace pend first
        if pend_found > 1 {
            lr_ind = pend_ind;
        } else {
            lr_ind = act_ind;
        }

        if let Some(idx) = lr_ind {
            if let Some(spilled_int) = get_int(vgic,
                vcpu.clone(),
                bit_extract(GICH.lr(idx), GICH_LR_VID_OFF, GICH_LR_VID_LEN),
            ) {
                if spilled_int.id() != interrupt.id() {
                    let spilled_int_lock = spilled_int.lock.lock();
                    remove_lr(vgic, vcpu.clone(), spilled_int.clone());
                    vgic_int_yield_owner(vcpu.clone(), spilled_int.clone());
                    drop(spilled_int_lock);
                } else {
                    remove_lr(vgic, vcpu.clone(), spilled_int.clone());
                    vgic_int_yield_owner(vcpu.clone(), spilled_int.clone());
                }
            }
        }
    }

    match lr_ind {
        Some(idx) => {
            write_lr(vgic, vcpu, interrupt, idx);
            return true;
        }
        None => {
            // turn on maintenance interrupts
            if vgic_get_state(interrupt) & IrqState::IrqSPend.to_num() != 0 {
                let hcr = GICH.hcr();
                //No Pending Interrupt Enable. Enables the signaling of a maintenance interrupt when there are no List registers with the State field set to 0b01
                // then a maintance interrupt will come
                GICH.set_hcr(hcr | GICH_HCR_NPIE_BIT);
            }
        }
    }

    false
}

fn write_lr(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>, lr_ind: usize) {
    let vcpu_id = vcpu.vcpu_id;
    let int_id = interrupt.id() as usize;
    let int_prio = interrupt.get_priority();

    let prev_int_id = vgic.cpu_priv_curr_lrs(vcpu_id, lr_ind) as usize;
    if prev_int_id != int_id && !gic_is_priv(prev_int_id) {
        if let Some(prev_interrupt) = get_int(vgic, vcpu.clone(), prev_int_id) {
            let prev_interrupt_lock = prev_interrupt.lock.lock();
            if vgic_owns(vcpu.clone(), prev_interrupt.clone())
                && prev_interrupt.in_lr()
                && (prev_interrupt.lr() == lr_ind as u16)
            {
                prev_interrupt.set_in_lr(false);
                vgic_int_yield_owner(vcpu.clone(), prev_interrupt.clone());
            }
            drop(prev_interrupt_lock);
        }
    }

    let state = vgic_get_state(interrupt.clone());

    let mut lr = (int_id << GICH_LR_VID_OFF) & GICH_LR_VID_MASK;
    lr |= ((int_prio as usize) << GICH_LR_PRIO_OFF) & GICH_LR_PRIO_MSK;

    if vgic_int_is_hw(interrupt.clone()) {
        lr |= GICH_LR_HW_BIT;
        lr |= (int_id << GICH_LR_PID_OFF) & GICH_LR_PID_MSK;
        if state == IrqState::IrqSPendActive.to_num() {
            lr |= GICH_LR_STATE_ACT;
        } else {
            lr |= (state << GICH_LR_STATE_OFF) & GICH_LR_STATE_MSK;
        }
    } else {
        if !gic_is_priv(int_id) && !vgic_int_is_hw(interrupt.clone()) {
            lr |= GICH_LR_EOI_BIT;
        }

        lr |= (state << GICH_LR_STATE_OFF) & GICH_LR_STATE_MSK;
    }

    /*
     * When the guest is using vGICv3, all the IRQs are Group 1. Group 0
     * would result in a FIQ, which will not be expected by the guest OS.
     */
    lr |= GICH_LR_GRP_BIT;

    interrupt.set_state(IrqState::IrqSInactive);
    interrupt.set_in_lr(true);
    interrupt.set_lr(lr_ind as u16);
    vgic.set_cpu_priv_curr_lrs(vcpu_id, lr_ind, int_id as u16);

    GICH.set_lr(lr_ind, lr);
    update_int_list(vgic, vcpu, interrupt);
}

fn route(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) {
    if let IrqState::IrqSInactive = interrupt.state() {
        return;
    }

    if !interrupt.enabled() {
        return;
    }

    /*  ==== vigc ===== */
    /*
    if vgic_int_vcpu_is_target(&vcpu, &interrupt) {
        add_lr(vgic, vcpu.clone(), interrupt.clone());
    }

    if !interrupt.in_lr() && vgic_int_has_other_target(interrupt.clone()) {
        let vcpu_vm_id = vcpu.vm_id();
        let ipi_msg = IpiInitcMessage {
            event: InitcEvent::VgicdRoute,
            vm_id: vcpu_vm_id,
            int_id: interrupt.id(),
            val: 0,
        };
        vgic_int_yield_owner(vcpu.clone(), interrupt.clone());
        let trglist = vgic_int_ptarget_mask(interrupt) & !(1 << vcpu.phys_id());
        for i in 0..PLAT_DESC.cpu_desc.num {
            if trglist & (1 << i) != 0 {
                ipi_send_msg(i, IpiType::IpiTIntc, IpiInnerMsg::Initc(ipi_msg));
            }
        }
    }
    */
    add_lr(vgic, vcpu.clone(), interrupt.clone());
}


