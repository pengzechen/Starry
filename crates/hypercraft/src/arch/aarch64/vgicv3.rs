#![allow(missing_docs, warnings)]

use core::mem::size_of;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

use spin::Mutex;

use super::vm::VM;

use super::vcpu::VCpu;
use crate::{GuestPageTableTrait, HyperCraftHal};

use arm_gicv3::*;
use crate::IrqState;

use core::marker::PhantomData;



/// GICv3 interrupt struct
#[derive(Clone)] pub struct VgicInt<H: HyperCraftHal, G: GuestPageTableTrait> {
    inner: Arc<Mutex<VgicIntInner<H, G>>>,
    /// lock
    pub lock: Arc<Mutex<()>>,
}

impl<H: HyperCraftHal, G: GuestPageTableTrait> VgicInt<H, G> {
    pub fn new(id: usize) -> VgicInt<H, G> {
        VgicInt {
            inner: Arc::new(Mutex::new(VgicIntInner::new(id))),
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn priv_new(id: usize, owner: VCpu<H>, targets: usize, enabled: bool, redist: usize, cfg: usize) -> VgicInt<H, G> {
        VgicInt {
            inner: Arc::new(Mutex::new(VgicIntInner::priv_new(
                id, owner, targets, enabled, redist, cfg,
            ))),
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn set_in_pend_state(&self, is_pend: bool) {
        let mut vgic_int = self.inner.lock();
        vgic_int.in_pend = is_pend;
    }

    pub fn set_in_act_state(&self, is_act: bool) {
        let mut vgic_int = self.inner.lock();
        vgic_int.in_act = is_act;
    }

    pub fn in_pend(&self) -> bool {
        let vgic_int = self.inner.lock();
        vgic_int.in_pend
    }

    pub fn in_act(&self) -> bool {
        let vgic_int = self.inner.lock();
        vgic_int.in_act
    }

    pub fn set_enabled(&self, enabled: bool) {
        let mut vgic_int = self.inner.lock();
        vgic_int.enabled = enabled;
    }

    pub fn set_lr(&self, lr: u16) {
        let mut vgic_int = self.inner.lock();
        vgic_int.lr = lr;
    }

    fn set_targets(&self, targets: u8) {
        let mut vgic_int = self.inner.lock();
        vgic_int.targets = targets;
    }

    pub fn set_prio(&self, prio: u8) {
        let mut vgic_int = self.inner.lock();
        vgic_int.prio = prio;
    }

    pub fn set_in_lr(&self, in_lr: bool) {
        let mut vgic_int = self.inner.lock();
        vgic_int.in_lr = in_lr;
    }

    pub fn set_state(&self, state: IrqState) {
        let mut vgic_int = self.inner.lock();
        vgic_int.state = state;
    }

    pub fn set_owner(&self, owner: VCpu<H>) {
        let mut vgic_int = self.inner.lock();
        vgic_int.owner = Some(owner);
    }

    pub fn clear_owner(&self) {
        let mut vgic_int = self.inner.lock();
        vgic_int.owner = None;
    }

    pub fn set_hw(&self, hw: bool) {
        let mut vgic_int = self.inner.lock();
        vgic_int.hw = hw;
    }

    pub fn set_cfg(&self, cfg: u8) {
        let mut vgic_int = self.inner.lock();
        vgic_int.cfg = cfg;
    }

    pub fn lr(&self) -> u16 {
        let vgic_int = self.inner.lock();
        vgic_int.lr
    }

    pub fn in_lr(&self) -> bool {
        let vgic_int = self.inner.lock();
        vgic_int.in_lr
    }

    pub fn route(&self) -> u64 {
        let vgic_int = self.inner.lock();
        vgic_int.route
    }

    pub fn phys_redist(&self) -> u64 {
        let vgic_int = self.inner.lock();
        match vgic_int.phys {
            VgicIntPhys::Redist(redist) => redist,
            _ => {
                panic!("must get redist!");
            }
        }
    }

    fn phys_route(&self) -> u64 {
        let vgic_int = self.inner.lock();
        match vgic_int.phys {
            VgicIntPhys::Route(route) => route,
            _ => {
                panic!("must get route!")
            }
        }
    }

    pub fn set_phys_route(&self, route: usize) {
        let mut vgic_int = self.inner.lock();
        vgic_int.phys = VgicIntPhys::Route(route as u64);
    }

    fn set_phys_redist(&self, redist: usize) {
        let mut vgic_int = self.inner.lock();
        vgic_int.phys = VgicIntPhys::Redist(redist as u64);
    }

    pub fn set_route(&self, route: usize) {
        let mut vgic_int = self.inner.lock();
        vgic_int.route = route as u64;
    }

    pub fn id(&self) -> u16 {
        let vgic_int = self.inner.lock();
        vgic_int.id
    }

    pub fn enabled(&self) -> bool {
        let vgic_int = self.inner.lock();
        vgic_int.enabled
    }

    pub fn prio(&self) -> u8 {
        let vgic_int = self.inner.lock();
        vgic_int.prio
    }

    fn targets(&self) -> u8 {
        let vgic_int = self.inner.lock();
        vgic_int.targets
    }

    pub fn hw(&self) -> bool {
        let vgic_int = self.inner.lock();
        vgic_int.hw
    }

    pub fn state(&self) -> IrqState {
        let vgic_int = self.inner.lock();
        vgic_int.state
    }

    pub fn cfg(&self) -> u8 {
        let vgic_int = self.inner.lock();
        vgic_int.cfg
    }

    pub fn owner(&self) -> Option<VCpu<H>> {
        let vgic_int = self.inner.lock();
        vgic_int.owner.as_ref().cloned()
    }

    pub fn owner_phys_id(&self) -> Option<usize> {
        let vgic_int = self.inner.lock();
        vgic_int.owner.as_ref().map(|owner| owner.pcpu_id)
    }

    fn owner_id(&self) -> Option<usize> {
        let vgic_int = self.inner.lock();
        match &vgic_int.owner {
            Some(owner) => Some(owner.vcpu_id),
            None => {
                error!("owner_id is None");
                None
            }
        }
    }

    fn owner_vm_id(&self) -> Option<usize> {
        let vgic_int = self.inner.lock();
        vgic_int.owner.as_ref().map(|owner| owner.vm_id)
    }

    // == pzc changed, original return : VM<H, G>
    fn owner_vm(&self) -> usize {
        let vgic_int = self.inner.lock();
        vgic_int.owner_vm()
    }
}



#[derive(Clone)] enum VgicIntPhys {  Redist(u64),  Route(u64),  }

pub struct VgicIntInner<H: HyperCraftHal, G: GuestPageTableTrait> {
    owner: Option<VCpu<H>>,
    route: u64,
    phys: VgicIntPhys,
    id: u16,
    hw: bool,
    in_lr: bool,
    lr: u16,
    enabled: bool,
    state: IrqState,
    prio: u8,
    targets: u8,
    cfg: u8,

    in_pend: bool,
    in_act: bool,

    marker: PhantomData<G>,
}

impl <H: HyperCraftHal, G: GuestPageTableTrait> VgicIntInner <H, G>{
    fn new(id: usize) -> VgicIntInner <H, G> {
        VgicIntInner {
            owner: None,
            route: GICD_IROUTER_INV as u64,
            phys: VgicIntPhys::Route(GICD_IROUTER_INV as u64),
            id: (id + GIC_PRIVATE_INT_NUM) as u16,
            hw: false,
            in_lr: false,
            lr: 0,
            enabled: false,
            state: IrqState::IrqSInactive,
            prio: 0xff,
            targets: 0,
            cfg: 0,
            in_pend: false,
            in_act: false,

            marker: PhantomData,
        }
    }

    fn priv_new(id: usize, owner: VCpu<H>, targets: usize, enabled: bool, redist: usize, cfg: usize) -> VgicIntInner<H, G> {
        VgicIntInner {
            owner: Some(owner),
            route: GICD_IROUTER_INV as u64,
            phys: VgicIntPhys::Redist(redist as u64),
            id: id as u16,
            hw: false,
            in_lr: false,
            lr: 0,
            enabled,
            state: IrqState::IrqSInactive,
            prio: 0xff,
            targets: targets as u8,
            cfg: cfg as u8,
            in_pend: false,
            in_act: false,

            marker: PhantomData,
        }
    }

    // == pzc changed, original return : VM<H, G>
    fn owner_vm(&self) -> usize {
        let owner = self.owner.as_ref().unwrap();
        // owner.vm().unwrap()
        owner.vm_id
    }
}




/// VGIC Distributor
pub struct Vgicd<H: HyperCraftHal, G: GuestPageTableTrait> {
    pub ctlr: u32,
    pub typer: u32,
    pub iidr: u32,
    pub interrupts: Vec<VgicInt<H, G>>,
}

impl<H: HyperCraftHal, G: GuestPageTableTrait> Vgicd <H, G>{
    fn default() -> Vgicd <H, G> {
        Vgicd {
            ctlr: 0,
            typer: 0,
            iidr: 0,
            interrupts: Vec::new(),
        }
    }
}


#[derive(Clone, Copy)] pub struct Sgis {
    pub pend: u8,
    pub act: u8,
}

impl Sgis {
    fn default() -> Sgis {
        Sgis { pend: 0, act: 0 }
    }
}


/// VGIC Redistributor
struct Vgicr {
    inner: Arc<Mutex<VgicrInner>>,
    pub lock: Arc<Mutex<()>>,
}

impl Vgicr {
    fn default() -> Vgicr {
        Vgicr {
            inner: Arc::new(Mutex::new(VgicrInner::default())),
            lock: Arc::new(Mutex::new(())),
        }
    }
    fn new(typer: usize, cltr: usize, iidr: usize) -> Vgicr {
        Vgicr {
            inner: Arc::new(Mutex::new(VgicrInner::new(typer, cltr, iidr))),
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn get_typer(&self) -> u64 {
        let vgicr = self.inner.lock();
        vgicr.typer
    }

    pub fn set_typer(&self, typer: usize) {
        let mut vgicr = self.inner.lock();
        vgicr.typer = typer as u64;
    }
}

struct VgicrInner {
    typer: u64,
    cltr: u32,
    iidr: u32,
}

impl VgicrInner {
    fn default() -> VgicrInner {
        VgicrInner {
            typer: 0,
            cltr: 0,
            iidr: 0,
        }
    }

    fn new(typer: usize, cltr: usize, iidr: usize) -> VgicrInner {
        VgicrInner {
            typer: typer as u64,
            cltr: cltr as u32,
            iidr: iidr as u32,
        }
    }
}



/// VGIC CPU Private data
pub struct  VgicCpuPriv <H: HyperCraftHal, G: GuestPageTableTrait>{
    vigcr: Vgicr,
    // gich: GicHypervisorInterfaceBlock,
    curr_lrs: [u16; GIC_LIST_REGS_NUM],
    sgis: [Sgis; GIC_SGIS_NUM],
    pub interrupts: Vec<VgicInt<H, G>>,

    pub pend_list: VecDeque<VgicInt<H, G>>,
    pub act_list: VecDeque<VgicInt<H, G>>,
}

impl <H: HyperCraftHal, G: GuestPageTableTrait> VgicCpuPriv <H, G>{
    pub fn default() -> VgicCpuPriv <H, G> {
        VgicCpuPriv {
            vigcr: Vgicr::default(),
            curr_lrs: [0; GIC_LIST_REGS_NUM],
            sgis: [Sgis::default(); GIC_SGIS_NUM],
            interrupts: Vec::new(),
            pend_list: VecDeque::new(),
            act_list: VecDeque::new(),
        }
    }

    pub fn new(typer: usize, cltr: usize, iidr: usize) -> VgicCpuPriv  <H, G> {
        VgicCpuPriv {
            vigcr: Vgicr::new(typer, cltr, iidr),
            curr_lrs: [0; GIC_LIST_REGS_NUM],
            sgis: [Sgis::default(); GIC_SGIS_NUM],
            interrupts: Vec::new(),
            pend_list: VecDeque::new(),
            act_list: VecDeque::new(),
        }
    }
}

/// VGIC general struct
pub struct Vgic <H: HyperCraftHal, G: GuestPageTableTrait> {
    pub vgicd: Mutex<Vgicd <H, G> >,
    pub cpu_priv: Mutex<Vec<VgicCpuPriv<H, G>>>,
}

impl <H: HyperCraftHal, G: GuestPageTableTrait> Vgic <H, G>{
    /// Return a default vigcV3
    pub fn default() -> Vgic <H, G> {
        Vgic {
            vgicd: Mutex::new(Vgicd::default()),
            cpu_priv: Mutex::new(Vec::new()),
        }
    }

    fn remove_int_list(&self, vcpu: VCpu<H>, interrupt: VgicInt<H, G>, is_pend: bool) {
        let mut cpu_priv = self.cpu_priv.lock();
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

    fn add_int_list(&self, vcpu: VCpu<H>, interrupt: VgicInt<H, G>, is_pend: bool) {
        let mut cpu_priv = self.cpu_priv.lock();
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

    fn update_int_list(&self, vcpu: VCpu<H>, interrupt: VgicInt<H, G>) {
        let state = interrupt.state().to_num();

        if state & IrqState::IrqSPend.to_num() != 0 && !interrupt.in_pend() {
            self.add_int_list(vcpu.clone(), interrupt.clone(), true);
        } else if state & IrqState::IrqSPend.to_num() == 0 {
            self.remove_int_list(vcpu.clone(), interrupt.clone(), true);
        }

        if state & IrqState::IrqSActive.to_num() != 0 && !interrupt.in_act() {
            self.add_int_list(vcpu.clone(), interrupt.clone(), false);
        } else if state & IrqState::IrqSActive.to_num() == 0 {
            self.remove_int_list(vcpu.clone(), interrupt.clone(), false);
        }
    }

    fn int_list_head(&self, vcpu: VCpu<H>, is_pend: bool) -> Option<VgicInt<H, G>> {
        let cpu_priv = self.cpu_priv.lock();
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

    /// Set vgicd ctlr
    pub fn set_vgicd_ctlr(&self, ctlr: u32) {
        let mut vgicd = self.vgicd.lock();
        vgicd.ctlr = ctlr;
    }
    
    /// Get vgicd ctlr
    pub fn vgicd_ctlr(&self) -> u32 {
        let vgicd = self.vgicd.lock();
        vgicd.ctlr
    }
    
    /// Get vgicd typer
    pub fn vgicd_typer(&self) -> u32 {
        let vgicd = self.vgicd.lock();
        vgicd.typer
    }
    
    /// Get vgicd iidr
    pub fn vgicd_iidr(&self) -> u32 {
        let vgicd = self.vgicd.lock();
        vgicd.iidr
    }

    /// Get cpu current private lr
    pub fn cpu_priv_curr_lrs(&self, cpu_id: usize, idx: usize) -> u16 {
        let cpu_priv = self.cpu_priv.lock();
        cpu_priv[cpu_id].curr_lrs[idx]
    }

    /// Set cpu current private lr
    pub fn set_cpu_priv_curr_lrs(&self, cpu_id: usize, idx: usize, val: u16) {
        let mut cpu_priv = self.cpu_priv.lock();
        cpu_priv[cpu_id].curr_lrs[idx] = val;
    }

    /// Get cpu sgis pending
    fn cpu_priv_sgis_pend(&self, cpu_id: usize, idx: usize) -> u8 {
        let cpu_priv = self.cpu_priv.lock();
        cpu_priv[cpu_id].sgis[idx].pend
    }

    /// Set cpu sgis pending
    fn set_cpu_priv_sgis_pend(&self, cpu_id: usize, idx: usize, pend: u8) {
        let mut cpu_priv = self.cpu_priv.lock();
        cpu_priv[cpu_id].sgis[idx].pend = pend;
    }

    /// Get cpu sgis active
    fn cpu_priv_sgis_act(&self, cpu_id: usize, idx: usize) -> u8 {
        let cpu_priv = self.cpu_priv.lock();
        cpu_priv[cpu_id].sgis[idx].act
    }

    /// Set cpu sgis active
    fn set_cpu_priv_sgis_act(&self, cpu_id: usize, idx: usize, act: u8) {
        let mut cpu_priv = self.cpu_priv.lock();
        cpu_priv[cpu_id].sgis[idx].act = act;
    }

    /// Get vgicd interrupt according to the index
    pub fn vgicd_interrupt(&self, idx: usize) -> VgicInt<H, G> {
        let vgicd = self.vgicd.lock();
        vgicd.interrupts[idx].clone()
    }

    /// Get cpu private interrupt according to the index
    pub fn cpu_priv_interrupt(&self, cpu_id: usize, idx: usize) -> VgicInt<H, G> {
        let cpu_priv = self.cpu_priv.lock();
        cpu_priv[cpu_id].interrupts[idx].clone()
    }

}