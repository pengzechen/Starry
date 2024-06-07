use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::marker::PhantomData;
use spin::Mutex;

use arm_gic::{GIC_LIST_REGS_NUM, GIC_PRIVATE_INT_NUM, GIC_SGIS_NUM};


use super::gic::IrqState;
use super::gic::*;

use super::utils::{bit_extract, bit_get, bit_set, bitmap_find_nth, ptr_read_write};
use super::vcpu::VCpu;
use super::vm::VM;
use crate::{GuestPageTableTrait, HyperCraftHal};

/// Vgic int inner struct
pub struct VgicIntInner<H: HyperCraftHal, G: GuestPageTableTrait> {
    /// interrupt owner
    pub owner: Option<VCpu<H>>,
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

impl<H: HyperCraftHal, G: GuestPageTableTrait> VgicIntInner<H, G> {
    fn new(id: usize) -> Self {
        Self {
            owner: None,
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

    /// Return a vgic int inner with owner
    fn priv_new(id: usize, owner: VCpu<H>, targets: usize, enabled: bool) -> Self {
        Self {
            owner: Some(owner),
            id: id as u16,
            hw: false,
            in_lr: false,
            lr: 0,
            enabled,
            state: IrqState::IrqSInactive,
            prio: 0xff,
            targets: targets as u8,
            cfg: 0,
            in_pend: false,
            in_act: false,

            marker: PhantomData,
        }
    }
}

/// Vgic int struct
#[derive(Clone)]
pub struct VgicInt<H: HyperCraftHal, G: GuestPageTableTrait> {
    inner: Arc<Mutex<VgicIntInner<H, G>>>,
    /// lock
    pub lock: Arc<Mutex<()>>,
}

impl<H: HyperCraftHal, G: GuestPageTableTrait> VgicInt<H, G> {
    /// Return a default vgic int inner
    pub fn new(id: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VgicIntInner::<H, G>::new(id))),
            lock: Arc::new(Mutex::new(())),
        }
    }
    /// Return a default vgic int inner with owner
    pub fn priv_new(id: usize, owner: VCpu<H>, targets: usize, enabled: bool) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VgicIntInner::<H, G>::priv_new(
                id, owner, targets, enabled,
            ))),
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Set interrupt pending state
    pub fn set_in_pend_state(&self, is_pend: bool) {
        let mut vgic_int = self.inner.lock();
        vgic_int.in_pend = is_pend;
    }

    /// set interrupt active state
    pub fn set_in_act_state(&self, is_act: bool) {
        let mut vgic_int = self.inner.lock();
        vgic_int.in_act = is_act;
    }

    /// Get interrupt wheter it is pending list
    pub fn in_pend(&self) -> bool {
        let vgic_int = self.inner.lock();
        vgic_int.in_pend
    }

    /// Get interrupt wheter it is active list
    pub fn in_act(&self) -> bool {
        let vgic_int = self.inner.lock();
        vgic_int.in_act
    }

    /// set the interrupt is enabled
    pub fn set_enabled(&self, enabled: bool) {
        let mut vgic_int = self.inner.lock();
        vgic_int.enabled = enabled;
    }

    /// set lr register index
    pub fn set_lr(&self, lr: u16) {
        let mut vgic_int = self.inner.lock();
        vgic_int.lr = lr;
    }

    /// set targets
    pub fn set_targets(&self, targets: u8) {
        let mut vgic_int = self.inner.lock();
        vgic_int.targets = targets;
    }

    /// get targets
    pub fn targets(&self) -> u8 {
        let vgic_int = self.inner.lock();
        vgic_int.targets
    }

    /// set interrupt priority
    pub fn set_priority(&self, prio: u8) {
        let mut vgic_int = self.inner.lock();
        vgic_int.prio = prio;
    }

    /// get interrupt priority
    pub fn get_priority(&self) -> u8 {
        let vgic_int = self.inner.lock();
        vgic_int.prio
    }

    /// set interrupt in lr
    pub fn set_in_lr(&self, in_lr: bool) {
        let mut vgic_int = self.inner.lock();
        vgic_int.in_lr = in_lr;
    }

    /// set interrupt state
    pub fn set_state(&self, state: IrqState) {
        let mut vgic_int = self.inner.lock();
        vgic_int.state = state;
    }

    /// set interrupt owner
    pub fn set_owner(&self, owner: VCpu<H>) {
        let mut vgic_int = self.inner.lock();
        vgic_int.owner = Some(owner);
    }

    /// clear interrupt owner
    pub fn clear_owner(&self) {
        let mut vgic_int = self.inner.lock();
        // println!("clear owner get lock");
        vgic_int.owner = None;
    }

    /// set wheter it is a hw interrupt
    pub fn set_hw(&self, hw: bool) {
        let mut vgic_int = self.inner.lock();
        vgic_int.hw = hw;
    }

    /// Set cfg register
    pub fn set_cfg(&self, cfg: u8) {
        let mut vgic_int = self.inner.lock();
        vgic_int.cfg = cfg;
    }

    /// Get lr register index
    pub fn lr(&self) -> u16 {
        let vgic_int = self.inner.lock();
        vgic_int.lr
    }

    /// Interrupt whether is in lr
    pub fn in_lr(&self) -> bool {
        let vgic_int = self.inner.lock();
        vgic_int.in_lr
    }

    /// Get interrupt id
    pub fn id(&self) -> u16 {
        let vgic_int = self.inner.lock();
        vgic_int.id
    }

    /// whether the interrupt is enabled
    pub fn enabled(&self) -> bool {
        let vgic_int = self.inner.lock();
        vgic_int.enabled
    }

    /// wheter the interrupt is hw
    pub fn hw(&self) -> bool {
        let vgic_int = self.inner.lock();
        vgic_int.hw
    }

    /// Get state of the interrupt
    pub fn state(&self) -> IrqState {
        let vgic_int = self.inner.lock();
        vgic_int.state
    }

    /// Get cfg register
    pub fn cfg(&self) -> u8 {
        let vgic_int = self.inner.lock();
        vgic_int.cfg
    }

    /// Get owner of the interrupt
    pub fn owner(&self) -> Option<VCpu<H>> {
        let vgic_int = self.inner.lock();
        match &vgic_int.owner {
            Some(vcpu) => {
                return Some(vcpu.clone());
            }
            None => {
                return None;
            }
        }
    }

    /// Get pcpu owner id of the interrupt
    pub fn owner_phys_id(&self) -> Option<usize> {
        let vgic_int = self.inner.lock();
        match &vgic_int.owner {
            Some(owner) => {
                return Some(owner.pcpu_id);
            }
            None => {
                return None;
            }
        }
    }

    /// Get vcpu owner id of the interrupt
    pub fn owner_id(&self) -> Option<usize> {
        let vgic_int = self.inner.lock();
        match &vgic_int.owner {
            Some(owner) => {
                return Some(owner.vcpu_id);
            }
            None => {
                return None;
            }
        }
    }

    /// Get vm owner id of the interrupt
    pub fn owner_vm_id(&self) -> Option<usize> {
        let vgic_int = self.inner.lock();
        match &vgic_int.owner {
            Some(owner) => {
                return Some(owner.vm_id);
            }
            None => {
                return None;
            }
        }
    }
}

/// virtual gicd
pub struct Vgicd<H: HyperCraftHal, G: GuestPageTableTrait> {
    /// control register
    pub ctlr: u32,
    /// typer
    pub typer: u32,
    /// implementer identification register
    pub iidr: u32,
    /// virtual interrupt list
    pub interrupts: Vec<VgicInt<H, G>>,
}

impl<H: HyperCraftHal, G: GuestPageTableTrait> Vgicd<H, G> {
    fn default() -> Self {
        Self {
            ctlr: 0,
            typer: 0,
            iidr: 0,
            interrupts: Vec::new(),
        }
    }
}

/// SGIs state
#[derive(Clone, Copy)]
pub struct Sgis {
    /// pending
    pub pend: u8,
    /// active
    pub act: u8,
}

impl Sgis {
    fn default() -> Sgis {
        Sgis { pend: 0, act: 0 }
    }
}

/// Vgic cpu private interrupt
pub struct VgicCpuPriv<H: HyperCraftHal, G: GuestPageTableTrait> {
    curr_lrs: [u16; GIC_LIST_REGS_NUM],
    /// SGIs state
    pub sgis: [Sgis; GIC_SGIS_NUM],
    /// interrupts list
    pub interrupts: Vec<VgicInt<H, G>>,
    /// pending list
    pub pend_list: VecDeque<VgicInt<H, G>>,
    /// active list
    pub act_list: VecDeque<VgicInt<H, G>>,
}

impl<H: HyperCraftHal, G: GuestPageTableTrait> VgicCpuPriv<H, G> {
    /// Return a default vgic cpu private interrupt
    pub fn default() -> Self {
        Self {
            curr_lrs: [0; GIC_LIST_REGS_NUM],
            sgis: [Sgis::default(); GIC_SGIS_NUM],
            interrupts: Vec::new(),
            pend_list: VecDeque::new(),
            act_list: VecDeque::new(),
        }
    }
}

/// Virtual GIC
pub struct Vgic<H: HyperCraftHal, G: GuestPageTableTrait> {
    /// virtual gicd
    pub vgicd: Mutex<Vgicd<H, G>>,
    /// virtual cpu private interrupt
    pub cpu_priv: Mutex<Vec<VgicCpuPriv<H, G>>>,
}

impl<H: HyperCraftHal, G: GuestPageTableTrait> Vgic<H, G> {
    /// Return a default vgic
    pub fn default() -> Self {
        Self {
            vgicd: Mutex::new(Vgicd::<H, G>::default()),
            cpu_priv: Mutex::new(Vec::new()),
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
    pub fn cpu_priv_sgis_pend(&self, cpu_id: usize, idx: usize) -> u8 {
        let cpu_priv = self.cpu_priv.lock();
        cpu_priv[cpu_id].sgis[idx].pend
    }

    /// Set cpu sgis pending
    pub fn set_cpu_priv_sgis_pend(&self, cpu_id: usize, idx: usize, pend: u8) {
        let mut cpu_priv = self.cpu_priv.lock();
        cpu_priv[cpu_id].sgis[idx].pend = pend;
    }

    /// Get cpu sgis active
    pub fn cpu_priv_sgis_act(&self, cpu_id: usize, idx: usize) -> u8 {
        let cpu_priv = self.cpu_priv.lock();
        cpu_priv[cpu_id].sgis[idx].act
    }
    
    /// Set cpu sgis active
    pub fn set_cpu_priv_sgis_act(&self, cpu_id: usize, idx: usize, act: u8) {
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

