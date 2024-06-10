

use hypercraft::arch::vgicv3::{Vgic, VgicInt};
use hypercraft::{GuestPageTableTrait, IrqState, VCpu, VM, };
use crate::{GuestPageTable, HyperCraftHalImpl};

use arm_gicv3::{
    GIC_PRIVATE_INT_NUM, GIC_INTS_MAX, GICH_LR_STATE_OFF, GICH_LR_STATE_LEN, GICH_HCR_NPIE_BIT,
    GICH_LR_VID_OFF, GICH_LR_VID_LEN, 
    // add_lr
    GICH_LR_PRIO_MSK, GICH_LR_PRIO_OFF, GICH_LR_STATE_MSK, GICH_LR_VID_MASK, GICH_LR_STATE_ACT, 
    GICH_LR_STATE_PND, 
    // vgic_int_is_hw
    GIC_SGIS_NUM,
    // write_lr
    GICH_LR_HW_BIT, GICH_LR_PID_OFF, GICH_LR_PID_MSK, GICH_LR_EOI_BIT, GICH_LR_GRP_BIT,  

    GICD_CTLR_ARE_NS_BIT,  GIC_CONFIG_BITS, GIC_PRIO_BITS, GICH_HCR_UIE_BIT, 

    //set irouter
    GICD_IROUTER_IRM_BIT, MPIDR_AFF_MSK, GICD_IROUTER_INV, GICD_IROUTER_RES0_MSK
};

use hypercraft::arch::utils::bitmap_find_nth;
use axhal::gicv3::gic_set_act;

use super::{active_vm, active_vm_id, current_cpu};
use hypercraft::arch::emu::EmuContext;

use arm_gicv3::GICH;
use arm_gicv3::GICR;
use arm_gicv3::GICD;
use arm_gicv3::cpuid2mpidr;
use arm_gicv3::platform::PLAT_DESC;

use arm_gicv3::gic_is_priv;
use hypercraft::arch::utils::bit_extract;
use hypercraft::arch::utils::bit_get;
use hypercraft::gicv3::gic_set_state;

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

// add interrupt in specific vcpu
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

// get interrupt in pend_list[0] or act_list[0]
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
    if int_id < GIC_PRIVATE_INT_NUM {
        let vcpu_id = vcpu.vcpu_id;
        return Some(vgic.cpu_priv_interrupt(vcpu_id, int_id));
    } else if (GIC_PRIVATE_INT_NUM..GIC_INTS_MAX).contains(&int_id) {
        return Some(vgic.vgicd_interrupt(int_id - GIC_PRIVATE_INT_NUM));
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
    let cpu_id = current_cpu().cpu_id;
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

    if interrupt.in_lr() && interrupt.owner_phys_id().unwrap() == current_cpu().cpu_id {
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
        let mut min_prio_act = interrupt.prio() as usize;
        let mut min_prio_pend = interrupt.prio() as usize;
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
    let int_prio = interrupt.prio();

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


// remove   !vgic_int_get_owner(vcpu.clone(), interrupt.clone())
fn set_enable(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>,
        int_id: usize, en: bool) {
    match get_int(vgic, vcpu.clone(), int_id) {
        Some(interrupt) => {
            let interrupt_lock = interrupt.lock.lock();
            //if vgic_int_get_owner(vcpu.clone(), interrupt.clone()) {
                
                if interrupt.enabled() ^ en {
                    interrupt.set_enabled(en);
                    remove_lr(vgic, vcpu.clone(), interrupt.clone());
                    if interrupt.hw() {
                        if gic_is_priv(interrupt.id() as usize) {
                            GICR.set_enable(interrupt.id() as usize, en, interrupt.phys_redist() as u32);
                        } else {
                            GICD.set_enable(interrupt.id() as usize, en);
                        }
                    }
                }
                route(vgic, vcpu.clone(), interrupt.clone());
                vgic_int_yield_owner(vcpu, interrupt.clone());
            /*
            } else {
                let int_phys_id = interrupt.owner_phys_id().unwrap();
                let vcpu_vm_id = vcpu.vm_id;
                let ipi_msg = IpiInitcMessage {
                    event: InitcEvent::VgicdSetEn,
                    vm_id: vcpu_vm_id,
                    int_id: interrupt.id(),
                    val: en as u8,
                };
                if !ipi_send_msg(int_phys_id, IpiType::IpiTIntc, IpiInnerMsg::Initc(ipi_msg)) {
                    error!(
                        "vgicd_set_enable: Failed to send ipi message, target {} type {}",
                        int_phys_id, 0
                    );
                }
            }
            */
            drop(interrupt_lock);
        }
        None => {
            error!("vgicd_set_enable: interrupt {} is illegal", int_id);
        }
    }
}

fn get_enable(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        int_id: usize) -> bool {
    get_int(vgic, vcpu, int_id).unwrap().enabled()
}


fn set_pend(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>,
        int_id: usize, pend: bool) {
    if let Some(interrupt) = get_int(vgic, vcpu.clone(), int_id) {
        let interrupt_lock = interrupt.lock.lock();
        // if vgic_int_get_owner(vcpu.clone(), interrupt.clone()) {
            remove_lr(vgic, vcpu.clone(), interrupt.clone());

            let state = interrupt.state().to_num();
            if pend && ((state & 1) == 0) {
                interrupt.set_state(IrqState::num_to_state(state | 1));
            } else if !pend && (state & 1) != 0 {
                interrupt.set_state(IrqState::num_to_state(state & !1));
            }

            let state = interrupt.state().to_num();
            if interrupt.hw() {
                if gic_is_priv(int_id) {
                    gic_set_state(interrupt.id() as usize, state, interrupt.phys_redist() as u32);
                } else {
                    // GICD non`t need gicr_id
                    gic_set_state(interrupt.id() as usize, state, 0);
                }
            }
            route(vgic, vcpu.clone(), interrupt.clone());
            vgic_int_yield_owner(vcpu, interrupt.clone());
        /*
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

                    if !ipi_send_msg(phys_id, IpiType::IpiTIntc, IpiInnerMsg::Initc(m)) {
                        error!(
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
        drop(interrupt_lock);
    }
}


fn set_active(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        int_id: usize, act: bool) {
    let interrupt_option = get_int(vgic, vcpu.clone(), bit_extract(int_id, 0, 10));
    if let Some(interrupt) = interrupt_option {
        let interrupt_lock = interrupt.lock.lock();
        // if vgic_int_get_owner(vcpu.clone(), interrupt.clone()) {
            remove_lr(vgic, vcpu.clone(), interrupt.clone());
            let state = interrupt.state().to_num();
            if act && ((state & IrqState::IrqSActive.to_num()) == 0) {
                interrupt.set_state(IrqState::num_to_state(state | IrqState::IrqSActive.to_num()));
            } else if !act && (state & IrqState::IrqSActive.to_num()) != 0 {
                interrupt.set_state(IrqState::num_to_state(state & !IrqState::IrqSActive.to_num()));
            }
            let state = interrupt.state().to_num();
            if interrupt.hw() {
                let vgic_int_id = interrupt.id() as usize;
                if gic_is_priv(vgic_int_id) {
                    gic_set_state(
                        vgic_int_id,
                        if state == 1 { 2 } else { state },
                        interrupt.phys_redist() as u32,
                    );
                } else {
                    gic_set_state(vgic_int_id, if state == 1 { 2 } else { state }, 0);
                }
            }
            route(vgic, vcpu.clone(), interrupt.clone());
            vgic_int_yield_owner(vcpu, interrupt.clone());
        /*
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
                error!(
                    "vgicd_set_active: Failed to send ipi message, target {} type {}",
                    phys_id, 0
                );
            }
        }
        */
        drop(interrupt_lock);
    }
}


fn set_icfgr(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        int_id: usize, cfg: u8) {
    if let Some(interrupt) = get_int(vgic, vcpu.clone(), int_id) {
        let interrupt_lock = interrupt.lock.lock();
        //if vgic_int_get_owner(vcpu.clone(), interrupt.clone()) {
            interrupt.set_cfg(cfg);
            if interrupt.hw() {
                if gic_is_priv(interrupt.id() as usize) {
                    GICR.set_icfgr(interrupt.id() as usize, cfg, interrupt.phys_redist() as u32);
                } else {
                    GICD.set_icfgr(interrupt.id() as usize, cfg);
                }
            }
            route(vgic, vcpu.clone(), interrupt.clone());
            vgic_int_yield_owner(vcpu, interrupt.clone());
        /*
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
                error!(
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

fn get_icfgr(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        int_id: usize) -> u8 {
    let interrupt_option = get_int(vgic, vcpu, int_id);
    if let Some(interrupt) = interrupt_option {
        interrupt.cfg()
    } else {
        unimplemented!();
    }
}

fn set_prio(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        int_id: usize, mut prio: u8) {
    let interrupt_option = get_int(vgic, vcpu.clone(), int_id);
    prio &= 0xf0; // gicv3 allows 8 priority bits in non-secure state

    if let Some(interrupt) = interrupt_option {
        let interrupt_lock = interrupt.lock.lock();
        // if vgic_int_get_owner(vcpu.clone(), interrupt.clone()) {
            if interrupt.prio() != prio {
                remove_lr(vgic, vcpu.clone(), interrupt.clone());
                let prev_prio = interrupt.prio();
                interrupt.set_prio(prio);
                if prio <= prev_prio {
                    route(vgic, vcpu.clone(), interrupt.clone());
                }
                if interrupt.hw() {
                    if gic_is_priv(interrupt.id() as usize) {
                        GICR.set_prio(interrupt.id() as usize, prio, interrupt.phys_redist() as u32);
                    } else {
                        GICD.set_prio(interrupt.id() as usize, prio);
                    }
                }
            }
            vgic_int_yield_owner(vcpu, interrupt.clone());
        /*
        } else {
            let vm_id = vcpu.vm_id();

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
                error!(
                    "set_prio: Failed to send ipi message, target {} type {}",
                    interrupt.owner_phys_id().unwrap(),
                    0
                );
            }
        }
        */
        drop(interrupt_lock);
    }
}

fn get_prio(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        int_id: usize) -> u8 {
    let interrupt_option = get_int(vgic, vcpu, int_id);
    interrupt_option.unwrap().prio()
}

pub fn vgic_inject(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>,
        int_id: usize) {
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


pub fn emu_ctrl_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>,  emu_ctx: &EmuContext) {
    if emu_ctx.write {
        let prev_ctlr = vgic.vgicd_ctlr();
        let idx = emu_ctx.reg;
        vgic.set_vgicd_ctlr(current_cpu().get_gpr(idx) as u32 & 0x2 | GICD_CTLR_ARE_NS_BIT as u32);
        // only one cpu for a vm, do not need ipi broadcast?
        /*
        if prev_ctlr ^ self.vgicd_ctlr() != 0 {
            let enable = self.vgicd_ctlr() != 0;
            let hcr = GICH.hcr();
            if enable {
                GICH.set_hcr(hcr | GICH_HCR_EN_BIT);
            } else {
                GICH.set_hcr(hcr & !GICH_HCR_EN_BIT);
            }

            let m = IpiInitcMessage {
                event: InitcEvent::VgicdGichEn,
                vm_id: active_vm_id(),
                int_id: 0,
                val: enable as u8,
            };
            ipi_intra_broadcast_msg(active_vm().unwrap(), IpiType::IpiTIntc, IpiInnerMsg::Initc(m));
        }
        */
    } else {
        let idx = emu_ctx.reg;
        let val = vgic.vgicd_ctlr() as usize;
        current_cpu().set_gpr(idx, val | GICD.ctlr() as usize);
    }
}

pub fn emu_typer_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>,  emu_ctx: &EmuContext) {
    if !emu_ctx.write {
        let idx = emu_ctx.reg;
        let val = vgic.vgicd_typer() as usize;
        current_cpu().set_gpr(idx, val);
    } else {
        error!("emu_typer_access: can't write to RO reg");
    }
}

pub fn emu_iidr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>,  emu_ctx: &EmuContext) {
    if !emu_ctx.write {
        let idx = emu_ctx.reg;
        let val = vgic.vgicd_iidr() as usize;
        current_cpu().set_gpr(idx, val);
    } else {
        error!("emu_iidr_access: can't write to RO reg");
    }
}


pub fn emu_isenabler_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    let first_int = ((emu_ctx.address & 0xffff) - 0x100) * 8; //emu_ctx.address - offsetof(GICD,ISENABLER)
    let idx = emu_ctx.reg;
    let mut val = if emu_ctx.write { current_cpu().get_gpr(idx) } else { 0 };

    let vm_id = active_vm_id();
    // let vm = match active_vm() {
    //     Some(vm) => vm,
    //     None => {
    //         panic!("emu_isenabler_access: current vcpu.vm is none");
    //     }
    // };
    let vm = active_vm();
    let mut vm_has_interrupt_flag = false;

    for i in 0..(emu_ctx.width * 8) {
        if vm.has_interrupt(first_int + i) || vm.emu_has_interrupt(first_int + i) {
            vm_has_interrupt_flag = true;
            break;
        }
    }
    if first_int >= 16 && !vm_has_interrupt_flag {
        error!(
            "emu_isenabler_access: vm[{}] does not have interrupt {}",
            vm_id, first_int
        );
        return;
    }

    if emu_ctx.write {
        for i in 0..(emu_ctx.width * 8) {
            if bit_get(val, i) != 0 {
                set_enable(vgic, current_cpu().active_vcpu.clone().unwrap(), first_int + i, true);
            }
        }
    } else {
        for i in 0..(emu_ctx.width * 8) {
            if get_enable(vgic, current_cpu().active_vcpu.clone().unwrap(), first_int + i) {
                val |= 1 << i;
            }
        }
        let idx = emu_ctx.reg;
        current_cpu().set_gpr(idx, val);
    }
}

pub fn emu_icenabler_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    let first_int = ((emu_ctx.address & 0xffff) - 0x0180) * 8; //emu_ctx.address - OFFSET(GICR/D,ICENABLE)
    let idx = emu_ctx.reg;
    let mut val = if emu_ctx.write { current_cpu().get_gpr(idx) } else { 0 };

    let vm_id = active_vm_id();
    // let vm = match active_vm() {
    //     Some(vm) => vm,
    //     None => {
    //         panic!("emu_activer_access: current vcpu.vm is none");
    //     }
    // };
    let vm = active_vm();
    let mut vm_has_interrupt_flag = false;

    if emu_ctx.write {
        for i in 0..32 {
            if vm.has_interrupt(first_int + i) || vm.emu_has_interrupt(first_int + i) {
                vm_has_interrupt_flag = true;
                break;
            }
        }
        if first_int >= 16 && !vm_has_interrupt_flag {
            warn!(
                "emu_icenabler_access: vm[{}] does not have interrupt {}",
                vm_id, first_int
            );
            return;
        }
    }

    if emu_ctx.write {
        for i in 0..(emu_ctx.width * 8) {
            if bit_get(val, i) != 0 {
                set_enable(vgic, current_cpu().active_vcpu.clone().unwrap(), first_int + i, false);
            }
        }
    } else {
        for i in 0..(emu_ctx.width * 8) {
            if get_enable(vgic, current_cpu().active_vcpu.clone().unwrap(), first_int + i) {
                val |= 1 << i;
            }
        }
        let idx = emu_ctx.reg;
        current_cpu().set_gpr(idx, val);
    }
}


pub fn emu_pendr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext, set: bool) {
    let first_int = if set {
        // ISPEND  emu_ctx.address - OFFSET(GICD/R,ISPENDR)
        ((emu_ctx.address & 0xffff) - 0x0200) * 8
    } else {
        // ICPEND  emu_ctx.address - OFFSET(GICD/R,ICPENDR)
        ((emu_ctx.address & 0xffff) - 0x0280) * 8
    };

    let idx = emu_ctx.reg;
    let mut val = if emu_ctx.write { current_cpu().get_gpr(idx) } else { 0 };

    if emu_ctx.write {
        for i in 0..(emu_ctx.width * 8) {
            if bit_get(val, i) != 0 {
                set_pend(vgic, current_cpu().active_vcpu.clone().unwrap(), first_int + i, set);
            }
        }
    } else {
        for i in 0..32 {
            match get_int(vgic, current_cpu().active_vcpu.clone().unwrap(), first_int + i) {
                Some(interrupt) => {
                    if vgic_get_state(interrupt.clone()) & IrqState::IrqSPend.to_num()
                        != IrqState::IrqSInactive.to_num()
                    {
                        val |= 1 << i;
                    }
                }
                None => {
                    unimplemented!();
                }
            }
        }
        let idx = emu_ctx.reg;
        current_cpu().set_gpr(idx, val);
    }
}

pub fn emu_icpendr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    emu_pendr_access(vgic, emu_ctx, false);
}

pub fn emu_ispendr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    emu_pendr_access(vgic, emu_ctx, true);
}


pub fn emu_activer_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext, set: bool) {
    let first_int = if set {
        // ISACTIVE (emu_ctx.address - OFFSET(GICD/R, ISACTIVER)
        8 * ((emu_ctx.address & 0xffff) - 0x0300)
    } else {
        // ICACTIVE (emu_ctx.address - OFFSET(GICD/R, ICACTIVER)
        8 * ((emu_ctx.address & 0xffff) - 0x0380)
    };
    let idx = emu_ctx.reg;

    let mut val = if emu_ctx.write { current_cpu().get_gpr(idx) } else { 0 };
    let vm_id = active_vm_id();
    // let vm = match active_vm() {
    //     Some(vm) => vm,
    //     None => {
    //         panic!("emu_activer_access: current vcpu.vm is none");
    //     }
    // };
    let vm = active_vm();
    let mut vm_has_interrupt_flag = false;

    for i in 0..(emu_ctx.width * 8) {
        if vm.has_interrupt(first_int + i) || vm.emu_has_interrupt(first_int + i) {
            vm_has_interrupt_flag = true;
            break;
        }
    }
    if first_int >= 16 && !vm_has_interrupt_flag {
        warn!(
            "emu_activer_access: vm[{}] does not have interrupt {}",
            vm_id, first_int
        );
        return;
    }

    if emu_ctx.write {
        for i in 0..(emu_ctx.width * 8) {
            if bit_get(val, i) != 0 {
                set_active(vgic, current_cpu().active_vcpu.clone().unwrap(), first_int + i, set);
            }
        }
    } else {
        for i in 0..(emu_ctx.width * 8) {
            match get_int(vgic, current_cpu().active_vcpu.clone().unwrap(), first_int + i) {
                Some(interrupt) => {
                    if vgic_get_state(interrupt.clone()) & IrqState::IrqSActive.to_num() != 0 {
                        val |= 1 << i;
                    }
                }
                None => {
                    unimplemented!();
                }
            }
        }
        let idx = emu_ctx.reg;
        current_cpu().set_gpr(idx, val);
    }
}

pub fn emu_isactiver_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    emu_activer_access(vgic, emu_ctx, true);
}

pub fn emu_icactiver_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    emu_activer_access(vgic, emu_ctx, false);
}
       

pub fn emu_icfgr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    let first_int = ((emu_ctx.address & 0xffff) - 0x0C00) * 8 / GIC_CONFIG_BITS; // emu_ctx.address - OFFSET(GICR/D,ICFGR)

    let vm_id = active_vm_id();
    // let vm = match active_vm() {
    //     Some(vm) => vm,
    //     None => {
    //         panic!("emu_icfgr_access: current vcpu.vm is none");
    //     }
    // };
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
            warn!("emu_icfgr_access: vm[{}] does not have interrupt {}", vm_id, first_int);
            return;
        }
    }

    if emu_ctx.write {
        let idx = emu_ctx.reg;
        let cfg = current_cpu().get_gpr(idx);
        let mut irq = first_int;
        let mut bit = 0;
        while bit < (emu_ctx.width * 8) {
            set_icfgr(vgic,
                current_cpu().active_vcpu.clone().unwrap(),
                irq,
                bit_extract(cfg as usize, bit, GIC_CONFIG_BITS) as u8,
            );
            bit += 2;
            irq += 1;
        }
    } else {
        let mut cfg = 0;
        let mut irq = first_int;
        let mut bit = 0;
        while bit < (emu_ctx.width * 8) {
            cfg |= (get_icfgr(vgic, current_cpu().active_vcpu.clone().unwrap(), irq) as usize) << bit;
            bit += 2;
            irq += 1;
        }
        let idx = emu_ctx.reg;
        current_cpu().set_gpr(idx, cfg);
    }
}

pub fn emu_ipriorityr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    let first_int = ((emu_ctx.address & 0xffff) - 0x0400) * 8 / GIC_PRIO_BITS; // emu_ctx.address - OFFSET(GICR/D,IPRIORITYR)
    let idx = emu_ctx.reg;
    let mut val = if emu_ctx.write { current_cpu().get_gpr(idx) } else { 0 };

    let vm_id = active_vm_id();
    // let vm = match active_vm() {
    //     Some(vm) => vm,
    //     None() => {
    //         panic!("emu_ipriorityr_access: current vcpu.vm is none");
    //     }
    // };
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
            warn!(
                "emu_ipriorityr_access: vm[{}] does not have interrupt {}",
                vm_id, first_int
            );
            return;
        }
    }

    if emu_ctx.write {
        for i in 0..emu_ctx.width {
            set_prio(vgic, 
                current_cpu().active_vcpu.clone().unwrap(),
                first_int + i,
                bit_extract(val, GIC_PRIO_BITS * i, GIC_PRIO_BITS) as u8,
            );
        }
    } else {
        for i in 0..emu_ctx.width {
            val |= (get_prio(vgic, current_cpu().active_vcpu.clone().unwrap(), first_int + i) as usize)
                << (GIC_PRIO_BITS * i);
        }
        let idx = emu_ctx.reg;
        current_cpu().set_gpr(idx, val);
    }
}

pub fn handle_trapped_eoir(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>) {
    let gic_lrs = gic_lrs();
    // eisr():Interrupt Controller End of Interrupt Status Register
    let mut lr_idx_opt = bitmap_find_nth(GICH.eisr() as usize, 0, gic_lrs, 1, true);

    while lr_idx_opt.is_some() {
        let lr_idx = lr_idx_opt.unwrap();
        let lr_val = GICH.lr(lr_idx) as usize;
        GICH.set_lr(lr_idx, 0);

        match get_int(vgic, vcpu.clone(), bit_extract(lr_val, GICH_LR_VID_OFF, GICH_LR_VID_LEN)) {
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
                continue;
            }
        }
        lr_idx_opt = bitmap_find_nth(GICH.eisr() as usize, 0, gic_lrs, 1, true);
    }
}


fn vgic_highest_proi_spilled(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: &VCpu<HyperCraftHalImpl>, 
        flag: bool) -> Option<VgicInt<HyperCraftHalImpl, GuestPageTable>> {
    let cpu_priv = &vgic.cpu_priv.lock()[vcpu.vcpu_id];
    // info!("binding list_len:{}", binding.len());
    let array = [
        Some(cpu_priv.pend_list.iter()),
        if flag { None } else { Some(cpu_priv.act_list.iter()) },
    ];
    let binding = array.into_iter().flatten().flatten();
    binding
        .min_by_key(|x| (((x.prio() as u32) << 10) | x.id() as u32))
        .cloned()
}

fn vgic_int_get_owner(vcpu: VCpu<HyperCraftHalImpl>, interrupt: VgicInt<HyperCraftHalImpl, GuestPageTable>) -> bool {
    let vcpu_id = vcpu.vcpu_id;
    let vcpu_vm_id = vcpu.vm_id;

    match interrupt.owner() {
        Some(owner) => {
            let owner_vcpu_id = owner.vcpu_id;
            let owner_vm_id = owner.vm_id;

            owner_vm_id == vcpu_vm_id && owner_vcpu_id == vcpu_id
        }
        None => {
            interrupt.set_owner(vcpu);
            true
        }
    }
}


pub fn refill_lrs(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>,  flag: bool) {
    let gic_lrs = gic_lrs();
    // ICH_ELRSR_EL2:locate a usable List register when the hypervisor is delivering an interrupt to a Guest OS.
    let mut lr_idx_opt = bitmap_find_nth(GICH.elrsr(), 0, gic_lrs, 1, true);
    // flag indicates that is no pending or not true:no pending flase:have pending,the we will look up active and pend
    let mut new_flags = flag;
    while lr_idx_opt.is_some() {
        let interrupt_opt: Option<VgicInt<HyperCraftHalImpl, GuestPageTable>> = vgic_highest_proi_spilled(vgic, &vcpu, new_flags);

        match interrupt_opt {
            Some(interrupt) => {
                let interrupt_lock = interrupt.lock.lock();
                let got_ownership = vgic_int_get_owner(vcpu.clone(), interrupt.clone());
                if got_ownership {
                    write_lr(vgic, vcpu.clone(), interrupt.clone(), lr_idx_opt.unwrap());
                }
                drop(interrupt_lock);
                if !got_ownership {
                    continue;
                }
            }
            None => {
                let hcr = GICH.hcr();
                GICH.set_hcr(hcr & !(GICH_HCR_NPIE_BIT | GICH_HCR_UIE_BIT));
                break;
            }
        }

        new_flags = false;
        lr_idx_opt = bitmap_find_nth(GICH.elrsr(), 0, gic_lrs, 1, true);
    }
}

pub fn eoir_highest_spilled_active(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>) {
    let cpu_priv = vgic.cpu_priv.lock();
    let binding = &cpu_priv[vcpu.vcpu_id].act_list;
    let interrupt = binding
        .iter()
        .min_by_key(|x| (((x.prio() as u32) << 10) | x.id() as u32))
        .cloned();
    drop(cpu_priv);
    if let Some(int) = interrupt {
        int.lock.lock();
        vgic_int_get_owner(vcpu.clone(), int.clone());

        let state = int.state().to_num();
        int.set_state(IrqState::num_to_state(state & !2));
        update_int_list(vgic, vcpu.clone(), int.clone());

        if vgic_int_is_hw(int.clone()) {
            gic_set_act(int.id() as usize, false, current_cpu().cpu_id as u32);
        } else if int.state().to_num() & 1 != 0 {
            add_lr(vgic, vcpu, int.clone());
        }
    }
}

pub fn vgic_set_hw_int(vm:&mut VM<HyperCraftHalImpl, GuestPageTable>, int_id: usize) {
    if int_id < GIC_SGIS_NUM {
        return;
    }

    // if !vm.has_vgic() {
    //     return;
    // }

    let vgic = vm.vgic();

    if int_id < GIC_PRIVATE_INT_NUM {
        for i in 0..vm.vcpu_num() {
            if let Some(interrupt) = get_int(&vgic, vm.vcpu(i).unwrap().clone(), int_id) {
                let interrupt_lock = interrupt.lock.lock();
                interrupt.set_hw(true);
                drop(interrupt_lock);
            }
        }
    } else if let Some(interrupt) = get_int(&vgic, vm.vcpu(0).unwrap().clone(), int_id) {
        let interrupt_lock = interrupt.lock.lock();
        interrupt.set_hw(true);
        drop(interrupt_lock);
    }
}



pub fn emu_razwi(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    if !emu_ctx.write {
        current_cpu().set_gpr(emu_ctx.reg, 0);
    }
}

pub fn emu_irouter_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    let first_int = (bit_extract(emu_ctx.address, 0, 16) - 0x6000) / 8;
    let idx = emu_ctx.reg;
    let mut val = if emu_ctx.write { current_cpu().get_gpr(idx) } else { 0 };

    if emu_ctx.write {
        vgicd_set_irouter(vgic, current_cpu().active_vcpu.clone().unwrap(), first_int, val);
    } else {
        if !gic_is_priv(first_int) {
            val = get_int(vgic, current_cpu().active_vcpu.clone().unwrap(), first_int)
                .unwrap()
                .route() as usize;
        }
        current_cpu().set_gpr(idx, val);
    }
}

pub fn emu_pidr_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    if !emu_ctx.write {
        current_cpu().set_gpr(emu_ctx.reg, GICD.id(((emu_ctx.address & 0xff) - 0xd0) / 4) as usize);
    }
}


pub fn vgicd_set_irouter(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, vcpu: VCpu<HyperCraftHalImpl>, 
        int_id: usize, val: usize) {
    if let Some(interrupt) = get_int(vgic, vcpu.clone(), int_id) {
        let interrupt_lock = interrupt.lock.lock();

        // if vgic_int_get_owner(vcpu.clone(), interrupt.clone()) {
            remove_lr(vgic, vcpu.clone(), interrupt.clone());

            // let phys_route = if (val & GICD_IROUTER_IRM_BIT) != 0 {
            //     cpuid2mpidr(vcpu.pcpu_id)
            // } else {
            //     match vcpu.vm().unwrap().get_vcpu_by_mpidr(val & MPIDR_AFF_MSK) {
            //         Some(vcpu) => cpuid2mpidr(vcpu.phys_id()) & MPIDR_AFF_MSK,
            //         _ => GICD_IROUTER_INV,
            //     }
            // };
            /* ========= I'm not sure ======== */
            let phys_route = cpuid2mpidr(vcpu.pcpu_id);
            
            interrupt.set_phys_route(phys_route);
            interrupt.set_route(val & GICD_IROUTER_RES0_MSK);
            if interrupt.hw() {
                GICD.set_route(int_id, phys_route);
            }
            route(vgic, vcpu.clone(), interrupt.clone());
            vgic_int_yield_owner(vcpu.clone(), interrupt.clone());
        /*
        } else {
            let m = IpiInitcMessage {
                event: InitcEvent::VgicdRoute,
                vm_id: vcpu.vm().unwrap().id(),
                int_id: interrupt.id(),
                val: val as u8,
            };
            if !ipi_send_msg(
                interrupt.owner().unwrap().phys_id(),
                IpiType::IpiTIntc,
                IpiInnerMsg::Initc(m),
            ) {
                print!(
                    "vgicd_set_irouter: Failed to send ipi message, target {} type {}",
                    interrupt.owner().unwrap().phys_id(),
                    0
                );
            }
        }
        */
        drop(interrupt_lock);
    }
}


// ================ vGICR ==============

pub fn vgicr_emul_typer_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext, vgicr_id: usize) {
    let cpu_priv = vgic.cpu_priv.lock();
    if !emu_ctx.write {
        current_cpu().set_gpr(emu_ctx.reg, cpu_priv[vgicr_id].vigcr.get_typer() as usize);
    }
}

pub fn emu_probaser_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    if emu_ctx.write {
        GICR.set_propbaser(current_cpu().cpu_id, current_cpu().get_gpr(emu_ctx.reg));
    } else {
        current_cpu().set_gpr(emu_ctx.reg, GICR.get_propbaser(current_cpu().cpu_id) as usize);
    }
}

pub fn emu_pendbaser_access(vgic: &Vgic<HyperCraftHalImpl, GuestPageTable>, emu_ctx: &EmuContext) {
    if emu_ctx.write {
        GICR.set_pendbaser(current_cpu().cpu_id, current_cpu().get_gpr(emu_ctx.reg));
    } else {
        current_cpu().set_gpr(emu_ctx.reg, GICR.get_pendbaser(current_cpu().cpu_id) as usize);
    }
}

// pzc changed here
pub fn vgicr_emul_pidr_access(emu_ctx: &EmuContext, vgicr_id: usize) {
    if !emu_ctx.write {
        // let pgicr_id = current_cpu()
        //     .active_vcpu
        //     .clone()
        //     .unwrap()
        //     .vm()
        //     .unwrap()
        //     .vcpuid_to_pcpuid(vgicr_id);
        let pgicr_id = active_vm().vcpuid_to_pcpuid(vgicr_id).unwrap();
        // if let Ok(pgicr_id) = pgicr_id {
            current_cpu().set_gpr(
                emu_ctx.reg,
                GICR.get_id(pgicr_id as u32, ((emu_ctx.address & 0xff) - 0xd0) / 4) as usize,
            );
        // }
    }
}


use core::mem::size_of;
use arm_gicv3::GicRedistributor;


#[inline(always)]
pub fn vgicr_get_id(emu_ctx: &EmuContext) -> u32 {
    // ((emu_ctx.address - axconfig::GICR_PADDR) / size_of::<GicRedistributor>()) as u32
    /* 这个地方要让nimbos觉得跑在qemu上 */
    ((emu_ctx.address - 0x80a_0000) / size_of::<GicRedistributor>()) as u32
}

pub fn vgicr_emul_ctrl_access(emu_ctx: &EmuContext) {
    if !emu_ctx.write {
        current_cpu().set_gpr(emu_ctx.reg, GICR.get_ctrl(current_cpu().cpu_id as u32) as usize);
    } else {
        GICR.set_ctrlr(current_cpu().cpu_id, current_cpu().get_gpr(emu_ctx.reg));
    }
}


// ==================== EMU reg ======================

/*
pub fn vgic_send_sgi_msg(vcpu: VCpu<HyperCraftHalImpl>, pcpu_mask: usize, int_id: usize) {
    let m = IpiInitcMessage {
        event: InitcEvent::Vgicdinject,
        vm_id: vcpu.vm().clone().unwrap().id(),
        int_id: int_id as u16,
        val: true as u8,
    };
    for i in 0..PLAT_DESC.cpu_desc.num {
        if (pcpu_mask & (1 << i)) != 0 {
            ipi_send_msg(i, IpiType::IpiTIntc, IpiInnerMsg::Initc(m));
        }
    }
}


pub fn vgic_icc_sgir_handler(_emu_dev_id: usize, emu_ctx: &EmuContext) -> bool {
    if emu_ctx.write {
        let sgir = current_cpu().get_gpr(emu_ctx.reg);
        let int_id = bit_extract(sgir, GICC_SGIR_SGIINTID_OFF, GICC_SGIR_SGIINTID_LEN);
        let targtlist = if (sgir & GICC_SGIR_IRM_BIT) != 0 {
            current_cpu().active_vcpu.clone().unwrap().vm().unwrap().ncpu() & !(1 << current_cpu().id)
        } else {
            let vm = match current_cpu().active_vcpu.clone().unwrap().vm() {
                Some(tvm) => tvm,
                None => {
                    panic!("vgic_icc_sgir_handler: current vcpu.vm is none");
                }
            };
            let mut vtarget = sgir & 0xffff;
            // maybe surrort more cluseter (aff1 != 0)
            if sgir & 0xff0000 != 0 && cfg!(feature = "rk3588") {
                //for rk3588 the aff1
                vtarget <<= (sgir & 0xf0000) >> 16;
            }
            vgic_target_translate(vm, vtarget as u32, true) as usize
        };
        vgic_send_sgi_msg(current_cpu().active_vcpu.clone().unwrap(), targtlist, int_id);
    }
    true
}
*/