extern crate alloc;

#[cfg(not(feature = "gic_v3"))]
use arm_gic::{
    GICD_TYPER_CPUNUM_MSK, GICD_TYPER_CPUNUM_OFF, GIC_PRIVATE_INT_NUM, GIC_SGIS_NUM
};
#[cfg(feature = "gic_v3")]
use arm_gicv3::{ GICD_TYPER_LPIS, GICD_TYPER_CPUNUM_OFF, MPIDR_AFF_MSK, GICR_TYPER_AFFVAL_OFF, GICR_TYPER_LAST_OFF, 
    GICD, GICR, GICH, GIC_PRIVATE_INT_NUM, GICH_MISR_EOI, GICH_MISR_NP, GICH_MISR_U, GICH_MISR_LRPEN, GICH_HCR_EOIC_OFF,
    GICD_TYPER_CPUNUM_MSK, GIC_SPI_MAX, GICR_TYPER_PRCNUM_OFF, GIC_SGIS_NUM, GICH_HCR_EOIC_MSK
};

#[cfg(not(feature = "gic_v3"))]
use hypercraft::arch::vgic::{Vgic, VgicInt, VgicCpuPriv};
#[cfg(feature = "gic_v3")]
use hypercraft::arch::vgicv3::{Vgic, VgicInt, VgicCpuPriv};

#[cfg(not(feature = "gic_v3"))]
use super::vgic::*;
#[cfg(feature = "gic_v3")]
use super::vgicv3::*;

use super::{active_vm, current_cpu};

#[cfg(not(feature = "gic_v3"))]
use axhal::{GIC_SPI_MAX, GICH, GICD};

use crate::{HyperCraftHalImpl, GuestPageTable};

use alloc::sync::Arc;
use hypercraft::VM;
use hypercraft::arch::emu::{EmuContext, EmuDevs};


const VGICD_REG_OFFSET_PREFIX_CTLR: usize = 0x0;
// same as TYPER & IIDR
const VGICD_REG_OFFSET_PREFIX_ISENABLER: usize = 0x2;
const VGICD_REG_OFFSET_PREFIX_ICENABLER: usize = 0x3;
const VGICD_REG_OFFSET_PREFIX_ISPENDR: usize = 0x4;
const VGICD_REG_OFFSET_PREFIX_ICPENDR: usize = 0x5;
const VGICD_REG_OFFSET_PREFIX_ISACTIVER: usize = 0x6;
const VGICD_REG_OFFSET_PREFIX_ICACTIVER: usize = 0x7;
const VGICD_REG_OFFSET_PREFIX_ICFGR: usize = 0x18;
const VGICD_REG_OFFSET_PREFIX_SGIR: usize = 0x1e;


/* 时间紧，先这样搞了，有时间再优化 -_- */


#[cfg(not(feature = "gic_v3"))]
pub fn emu_intc_handler(_emu_dev_id: usize, emu_ctx: &EmuContext) -> bool {
    // get the 0 to 11th bit of address, because gicd offset end in 0x1000
    let offset = emu_ctx.address & 0xfff;
    // max width bit is 0b11 (0b11 Doubleword)
    if emu_ctx.width > 4 {
        return false;
    }

    let vm = active_vm();
    let vgic = vm.vgic();
    // extract the 7th to 11th bit in offset in order to get the prefix of different registers
    let vgicd_offset_prefix = (offset & 0xf80) >> 7;

    if !vgicd_emu_access_is_vaild(emu_ctx) {
        return false;
    }

    match vgicd_offset_prefix {
        VGICD_REG_OFFSET_PREFIX_ISENABLER => {
            emu_isenabler_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_ISPENDR => {
            emu_ispendr_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_ISACTIVER => {
            emu_isactiver_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_ICENABLER => {
            emu_icenabler_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_ICPENDR => {
            emu_icpendr_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_ICACTIVER => {
            emu_icactiver_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_ICFGR => {
            emu_icfgr_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_SGIR => {
            emu_sgiregs_access(&*vgic, emu_ctx);
        }
        _ => {
            match offset {
                // VGICD_REG_OFFSET(CTLR)
                0 => {
                    emu_ctrl_access(&*vgic, emu_ctx);
                }
                // VGICD_REG_OFFSET(TYPER)
                0x004 => {
                    emu_typer_access(&*vgic, emu_ctx);
                }
                // VGICD_REG_OFFSET(IIDR)
                0x008 => {
                    emu_iidr_access(&*vgic, emu_ctx);
                }
                _ => {
                    if !emu_ctx.write {
                        let idx = emu_ctx.reg;
                        let val = 0;
                        current_cpu().set_gpr(idx, val);
                    }
                }
            }
            if offset >= 0x400 && offset < 0x800 {
                emu_ipriorityr_access(&*vgic, emu_ctx);
            } else if offset >= 0x800 && offset < 0xc00 {
                emu_itargetr_access(&*vgic, emu_ctx);
            }
        }
    }
    true
}

#[cfg(feature = "gic_v3")]
pub fn emu_intc_handler(_emu_dev_id: usize, emu_ctx: &EmuContext) -> bool {
    let offset = emu_ctx.address & 0xffff;

    // let vm = match active_vm() {
    //     None => {
    //         panic!("emu_intc_handler: vm is None");
    //     }
    //     Some(x) => x,
    // };
    let vm = active_vm();

    let vgic = vm.vgic();
    let vgicd_offset_prefix = offset >> 7;
    trace!(
        "current_cpu:{} emu_intc_handler offset:{:#x} is write:{},val:{:#x}",
        current_cpu().cpu_id,
        emu_ctx.address,
        emu_ctx.write,
        current_cpu().get_gpr(emu_ctx.reg)
    );

    match vgicd_offset_prefix {
        VGICD_REG_OFFSET_PREFIX_ISENABLER => {
            emu_isenabler_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_ISPENDR => {
            emu_ispendr_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_ISACTIVER => {
            emu_isactiver_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_ICENABLER => {
            emu_icenabler_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_ICPENDR => {
            emu_icpendr_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_ICACTIVER => {
            emu_icactiver_access(&*vgic, emu_ctx);
        }
        VGICD_REG_OFFSET_PREFIX_ICFGR => {
            emu_icfgr_access(&*vgic, emu_ctx);
        }
        _ => {
            match offset {
                // VGICD_REG_OFFSET(CTLR)
                0 => {
                    emu_ctrl_access(&*vgic, emu_ctx);
                }
                // VGICD_REG_OFFSET(TYPER)
                0x004 => {
                    emu_typer_access(&*vgic, emu_ctx);
                }
                // VGICD_REG_OFFSET(IIDR)
                0x008 => {
                    emu_iidr_access(&*vgic, emu_ctx);
                }
                0xf00 => {
                    emu_razwi(&*vgic, emu_ctx);
                }
                _ => {
                    if (0x400..0x800).contains(&offset) {
                        emu_ipriorityr_access(&*vgic, emu_ctx);
                    } else if (0x800..0xc00).contains(&offset) {
                        emu_razwi(&*vgic, emu_ctx);
                    } else if (0x6000..0x8000).contains(&offset) {
                        emu_irouter_access(&*vgic, emu_ctx);
                    } else if (0xffd0..0x10000).contains(&offset) {
                        //ffe8 is GICD_PIDR2, Peripheral ID2 Register
                        emu_pidr_access(&*vgic, emu_ctx);
                    } else {
                        emu_razwi(&*vgic, emu_ctx);
                    }
                }
            }
        }
    }
    true
}

#[cfg(not(feature = "gic_v3"))]
pub fn emu_intc_init(vm: &mut VM<HyperCraftHalImpl, GuestPageTable>, emu_dev_id: usize) {
    // let vgic_cpu_num = vm.config().cpu_num();
    // vm.init_intc_mode(true);

    let vgic_cpu_num = 1;
    let vgic = Arc::new(Vgic::<HyperCraftHalImpl, GuestPageTable>::default());

    let mut vgicd = vgic.vgicd.lock();
    vgicd.typer = (GICD.lock().get_typer() & GICD_TYPER_CPUNUM_MSK as u32)
        | (((vm.vcpu_num() - 1) << GICD_TYPER_CPUNUM_OFF) & GICD_TYPER_CPUNUM_MSK) as u32;
    vgicd.iidr = GICD.lock().get_iidr();

    for i in 0..GIC_SPI_MAX {
        vgicd.interrupts.push(VgicInt::<HyperCraftHalImpl, GuestPageTable>::new(i));
    }
    drop(vgicd);

    for i in 0..vgic_cpu_num {
        let mut cpu_priv = VgicCpuPriv::default();
        for int_idx in 0..GIC_PRIVATE_INT_NUM {
            let vcpu = vm.vcpu(i).unwrap();
            let phys_id = vcpu.pcpu_id;

            cpu_priv.interrupts.push(VgicInt::<HyperCraftHalImpl, GuestPageTable>::priv_new(
                int_idx,
                vcpu.clone(),
                1 << phys_id,
                int_idx < GIC_SGIS_NUM,
            ));
        }

        let mut vgic_cpu_priv = vgic.cpu_priv.lock();
        vgic_cpu_priv.push(cpu_priv);
    }

    vm.set_emu_devs(emu_dev_id, EmuDevs::<HyperCraftHalImpl, GuestPageTable>::Vgic(vgic.clone()));
}

#[cfg(feature = "gic_v3")]
pub fn emu_intc_init(vm: &mut VM<HyperCraftHalImpl, GuestPageTable>, emu_dev_id: usize) {
    // let vgic_cpu_num = vm.config().cpu_num();
    // vm.init_intc_mode(true);

    let vgic_cpu_num = 1;
    

    let vgic = Arc::new(Vgic::<HyperCraftHalImpl, GuestPageTable>::default());

    let mut vgicd = vgic.vgicd.lock();
    vgicd.typer = (GICD.typer() & !(GICD_TYPER_CPUNUM_MSK | GICD_TYPER_LPIS) as u32)
        | ((((vm.vcpu_num() - 1) << GICD_TYPER_CPUNUM_OFF) & GICD_TYPER_CPUNUM_MSK) as u32);
    vgicd.iidr = GICD.iidr();
    vgicd.ctlr = 0b10;

    for i in 0..GIC_SPI_MAX {
        vgicd.interrupts.push(VgicInt::<HyperCraftHalImpl, GuestPageTable>::new(i));
    }
    drop(vgicd);

    for i in 0..vgic_cpu_num {
        let mut typer = i << GICR_TYPER_PRCNUM_OFF;
        let vmpidr = vm.vcpu(i).unwrap().get_vmpidr();
        typer |= (vmpidr & MPIDR_AFF_MSK) << GICR_TYPER_AFFVAL_OFF;
        typer |= !!((i == (vm.vcpu_num() - 1)) as usize) << GICR_TYPER_LAST_OFF;
        //need the low 6 bits for LPI/ITS init
        //DPGS, bit [5]:Sets support for GICR_CTLR.DPG* bits
        //DirectLPI, bit [3]: Indicates whether this Redistributor supports direct injection of LPIs.
        //Dirty, bit [2]: Controls the functionality of GICR_VPENDBASER.Dirty.
        //LPI VLPIS, bit [1]: Indicates whether the GIC implementation supports virtual LPIs and the direct injection of virtual LPIs
        //PLPIS, bit [0]: Indicates whether the GIC implementation supports physical LPIs
        typer |= 0b10_0001;

        let mut cpu_priv = VgicCpuPriv::<HyperCraftHalImpl, GuestPageTable>::new(
            typer,
            GICR.get_ctrl(vm.vcpu(i).unwrap().pcpu_id as u32) as usize,
            GICR.get_iidr(vm.vcpu(i).unwrap().pcpu_id) as usize,
        );

        for int_idx in 0..GIC_SGIS_NUM {
            let vcpu = vm.vcpu(i).unwrap();
            let phys_id = vcpu.pcpu_id;

            cpu_priv.interrupts.push(VgicInt::<HyperCraftHalImpl, GuestPageTable>::priv_new(
                int_idx,
                vcpu.clone(),
                1 << phys_id,
                true,
                phys_id,
                0b10,
            ));
        }

        for int_idx in GIC_SGIS_NUM..GIC_PRIVATE_INT_NUM {
            let vcpu = vm.vcpu(i).unwrap();
            let phys_id = vcpu.pcpu_id;

            cpu_priv.interrupts.push(VgicInt::<HyperCraftHalImpl, GuestPageTable>::priv_new(
                int_idx,
                vcpu.clone(),
                1 << phys_id,
                false,
                phys_id,
                0b0,
            ));
        }

        let mut vgic_cpu_priv = vgic.cpu_priv.lock();
        vgic_cpu_priv.push(cpu_priv);
        drop(vgic_cpu_priv);
    }

    vm.set_emu_devs(emu_dev_id, EmuDevs::Vgic(vgic.clone()));
}

#[cfg(not(feature = "gic_v3"))]
pub fn vgicd_emu_access_is_vaild(emu_ctx: &EmuContext) -> bool {
    let offset = emu_ctx.address & 0xfff;
    let offset_prefix = (offset & 0xf80) >> 7;
    match offset_prefix {
        VGICD_REG_OFFSET_PREFIX_CTLR
        | VGICD_REG_OFFSET_PREFIX_ISENABLER
        | VGICD_REG_OFFSET_PREFIX_ISPENDR
        | VGICD_REG_OFFSET_PREFIX_ISACTIVER
        | VGICD_REG_OFFSET_PREFIX_ICENABLER
        | VGICD_REG_OFFSET_PREFIX_ICPENDR
        | VGICD_REG_OFFSET_PREFIX_ICACTIVER
        | VGICD_REG_OFFSET_PREFIX_ICFGR => {
            if emu_ctx.width != 4 || emu_ctx.address & 0x3 != 0 {
                return false;
            }
        }
        VGICD_REG_OFFSET_PREFIX_SGIR => {
            if (emu_ctx.width == 4 && emu_ctx.address & 0x3 != 0)
                || (emu_ctx.width == 2 && emu_ctx.address & 0x1 != 0)
            {
                return false;
            }
        }
        _ => {
            // TODO: hard code to rebuild (gicd IPRIORITYR and ITARGETSR)
            if offset >= 0x400 && offset < 0xc00 {
                if (emu_ctx.width == 4 && emu_ctx.address & 0x3 != 0)
                    || (emu_ctx.width == 2 && emu_ctx.address & 0x1 != 0)
                {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(feature = "gic_v3")]
pub fn vgicd_emu_access_is_vaild(emu_ctx: &EmuContext) -> bool {
    let offset = emu_ctx.address & 0xffff;
    let offset_prefix = (offset & 0xff80) >> 7;
    match offset_prefix {
        VGICD_REG_OFFSET_PREFIX_CTLR
        | VGICD_REG_OFFSET_PREFIX_ISENABLER
        | VGICD_REG_OFFSET_PREFIX_ISPENDR
        | VGICD_REG_OFFSET_PREFIX_ISACTIVER
        | VGICD_REG_OFFSET_PREFIX_ICENABLER
        | VGICD_REG_OFFSET_PREFIX_ICPENDR
        | VGICD_REG_OFFSET_PREFIX_ICACTIVER
        | VGICD_REG_OFFSET_PREFIX_ICFGR => {
            if emu_ctx.width != 4 || emu_ctx.address & 0x3 != 0 {
                return false;
            }
        }
        VGICD_REG_OFFSET_PREFIX_SGIR => {
            if (emu_ctx.width == 4 && emu_ctx.address & 0x3 != 0) || (emu_ctx.width == 2 && emu_ctx.address & 0x1 != 0)
            {
                return false;
            }
        }
        _ => {
            // TODO: hard code to rebuild (gicd IPRIORITYR and ITARGETSR)
            if (0x400..0xc00).contains(&offset)
                && ((emu_ctx.width == 4 && emu_ctx.address & 0x3 != 0)
                    || (emu_ctx.width == 2 && emu_ctx.address & 0x1 != 0))
            {
                return false;
            }
        }
    }
    true
}


#[cfg(not(feature = "gic_v3"))]
pub fn gic_maintenance_handler() {
    let misr = GICH.get_misr();
    let vm = active_vm();

    let vgic = vm.vgic();
    // 0b1 End Of Interrupt maintenance interrupt asserted.
    if misr & 1 != 0 {
        handle_trapped_eoir(&*vgic, current_cpu().get_active_vcpu().unwrap().clone());
    }

    // 0b1 List Register Entry Not Present maintenance interrupt asserted.
    if misr & (1 << 2) != 0 {
        let mut hcr = GICH.get_hcr();
        // deal with eoi
        while hcr & (0b11111 << 27) != 0 {
            eoir_highest_spilled_active(&*vgic, current_cpu().get_active_vcpu().unwrap().clone());
            hcr -= 1 << 27;
            GICH.set_hcr(hcr);
            hcr = GICH.get_hcr();
        }
    }

    // 0b1 No Pending maintenance interrupt asserted.
    if misr & (1 << 3) != 0 {
        refill_lrs(&*vgic, current_cpu().get_active_vcpu().unwrap().clone());
    }
}

#[cfg(feature = "gic_v3")]
pub fn gic_maintenance_handler() {
    let misr = GICH.misr();
    // let vm = match active_vm() {
    //     Some(vm) => vm,
    //     None => {
    //         panic!("gic_maintenance_handler: current vcpu.vm is None");
    //     }
    // };
    let vm = active_vm();
    let vgic = vm.vgic();

    if misr & (GICH_MISR_EOI as u32) != 0 {
        handle_trapped_eoir(&vgic, current_cpu().active_vcpu.clone().unwrap());
    }

    // NP:List Register Entry Not Present.
    // U: underflow Zero or one of the List register entries are marked as a valid interrupt, that is, if the corresponding ICH_LR<n>_EL2.State bits do not equal 0x0.
    if misr & (GICH_MISR_NP as u32 | GICH_MISR_U as u32) != 0 {
        refill_lrs(&vgic, 
            current_cpu().active_vcpu.clone().unwrap(),
            (misr & GICH_MISR_NP as u32) != 0,
        );
    }

    if misr & (GICH_MISR_LRPEN as u32) != 0 {
        let mut hcr = GICH.hcr();
        while hcr & GICH_HCR_EOIC_MSK != 0 {
            eoir_highest_spilled_active(&vgic, current_cpu().active_vcpu.clone().unwrap());
            hcr -= 1 << GICH_HCR_EOIC_OFF;
            GICH.set_hcr(hcr);
            hcr = GICH.hcr();
        }
    }
}

