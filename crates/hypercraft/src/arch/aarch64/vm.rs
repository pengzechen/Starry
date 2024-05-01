extern crate alloc;
use alloc::boxed::Box;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use crate::arch::VCpu;
use crate::{GuestPageTableTrait, HyperCraftHal, HyperResult, VcpusArray, VmCpus};

use super::emu::*;
use super::utils::*;
use super::vgic::*;
use super::vuart::*;

const VGIC_DEV_ID: usize = 0;
const UART_DEV_ID: usize = 1;

/// The guest VM
#[repr(align(4096))]
#[derive(Clone)]
pub struct VM<H: HyperCraftHal, G: GuestPageTableTrait> {
    /// The vcpus belong to VM
    pub vcpus: VcpusArray<H>,
    /// The guest page table of VM
    gpt: G,
    /// VM id
    pub vm_id: usize,
    /// interrupt
    pub intc_dev_id: usize,
    /// interrupt bitmap
    pub int_bitmap: Option<BitMap<BitAlloc256>>,
    /// emul devs
    pub emu_devs: Vec<EmuDevs<H, G>>,
}

impl<H: HyperCraftHal, G: GuestPageTableTrait> VM<H, G> {
    /// Create a new VM
    pub fn new(vcpus: VcpusArray<H>, gpt: G, id: usize) -> HyperResult<Self> {
        Ok(Self {
            vcpus: vcpus,
            gpt: gpt,
            vm_id: id,

            intc_dev_id: 0,
            int_bitmap: Some(BitAlloc4K::default()),
            emu_devs: Vec::new(),
        })
    }

    /// get vm vcpu by index
    pub fn vcpu(&self, idx: usize) -> Option<&VCpu<H>> {
        self.vcpus.get_vcpu(idx)
    }

    /// get vm vcpu by index
    pub fn vcpu_mut(&mut self, idx: usize) -> Option<&mut VCpu<H>> {
        self.vcpus.get_vcpu_mut(idx)
    }

    /// Init VM vcpu by vcpu id. Set kernel entry point.
    pub fn init_vm_vcpu(
        &mut self,
        vcpu_id: usize,
        kernel_entry_point: usize,
        device_tree_ipa: usize,
    ) {
        let vcpu = self.vcpus.get_vcpu_mut(vcpu_id).unwrap();
        // debug!("vm:{:#?}", vcpu.regs.vm_system_regs);
        vcpu.init(kernel_entry_point, device_tree_ipa);
    }

    /// Add a vcpu to VM
    pub fn add_vm_vcpu(&mut self, vcpu: VCpu<H>) {
        self.vcpus.add_vcpu(vcpu).unwrap();
    }

    /// Init VM vcpus. Set kernel entry point.
    pub fn init_vm_vcpus(&mut self, kernel_entry_point: usize, device_tree_ipa: usize) {
        for i in 0..self.vcpus.length {
            debug!("this is {} vcpu", i);
            self.init_vm_vcpu(i, kernel_entry_point, device_tree_ipa);
        }
    }

    /// Run this self.
    pub fn run(&mut self, vcpu_id: usize) {
        let vcpu = self.vcpus.get_vcpu(vcpu_id).unwrap();
        debug!("run vcpu{}", vcpu.vcpu_id);
        // debug!("vcpu: {:?}", vcpu.regs);
        let vttbr_token = (self.vm_id << 48) | self.gpt.token();
        debug!("vttbr_token: 0x{:X}", self.gpt.token());
        vcpu.run(vttbr_token);
    }

    /// Get vm vgic
    pub fn vgic(&self) -> Arc<Vgic<H, G>> {
        match &self.emu_devs[VGIC_DEV_ID] {
            EmuDevs::<H, G>::Vgic(vgic) => {
                return vgic.clone();
            }
            _ => {
                panic!("vm{} cannot find vgic", self.vm_id);
            }
        }
    }

    /// Get vm vgic
    pub fn vuart(&self) -> &Vuart {
        match &self.emu_devs[UART_DEV_ID] {
            EmuDevs::<H, G>::Vuart(vuart) => {
                return vuart;
            }
            _ => {
                panic!("vm{} cannot find vuart", self.vm_id);
            }
        }
    }

    /// Get vm vgic
    pub fn vuart_mut(&mut self) -> &mut Vuart {
        match &mut self.emu_devs[UART_DEV_ID] {
            EmuDevs::<H, G>::Vuart(vuart) => {
                return vuart;
            }
            _ => {
                panic!("vm{} cannot find vuart", self.vm_id);
            }
        }
    }
    
    /// Set vm emulated device by index
    pub fn set_emu_devs(&mut self, idx: usize, emu: EmuDevs<H, G>) {
        if idx < self.emu_devs.len() {
            if let EmuDevs::<H, G>::None = self.emu_devs[idx] {
                self.emu_devs[idx] = emu;
                return;
            } else {
                panic!("set_emu_devs: set an exsit emu dev");
            }
        }
        self.emu_devs.resize(idx, EmuDevs::<H, G>::None);
        self.emu_devs.push(emu);
    }

    /// Get cpu number of the vm
    pub fn vcpu_num(&self) -> usize {
        return self.vcpus.length;
    }

    /// Set interrupt id in the bitmap
    pub fn set_int_bit_map(&mut self, int_id: usize) {
        self.int_bitmap.as_mut().unwrap().set(int_id);
    }

    /// Judge if the interrupt id is in the bitmap
    pub fn has_interrupt(&self, int_id: usize) -> bool {
        self.int_bitmap.as_ref().unwrap().get(int_id) != 0
    }

    /// Judge if the interrupt id belongs to a emulated device
    pub fn emu_has_interrupt(&self, int_id: usize) -> bool {
        // hardcode for gicd
        if int_id == 0 {
            return true;
        }
        false
        /*
        for emu_dev in self.config().emulated_device_list() {
            if int_id == emu_dev.irq_id {
                return true;
            }
        }
        false
        */
    }

    /// Get the device id of the emulated device
    pub fn set_intc_dev_id(&mut self, idx: usize) {
        self.intc_dev_id = idx;
    }

     /// Change vcpu mask to pcpu mask
    pub fn vcpu_to_pcpu_mask(&self, mask: usize, len: usize) -> usize {
        let mut pmask = 0;
        for i in 0..len {
            if let Some(shift) = self.vcpuid_to_pcpuid(i) {
                if mask & (1 << i) != 0 {
                    pmask |= 1 << shift;
                }
            }
        }
        return pmask;
    }
    
    /// Change pcpu mask to vcpu mask
    pub fn pcpu_to_vcpu_mask(&self, mask: usize, len: usize) -> usize {
        let mut vmask = 0;
        for i in 0..len {
            if let Some(shift) = self.pcpuid_to_vcpuid(i) {
                if mask & (1 << i) != 0 {
                    vmask |= 1 << shift;
                }
            }
        }
        return vmask;
    }

    fn vcpuid_to_pcpuid(&self, vcpuid: usize) -> Option<usize> {
        // debug!("vcpuid_to_pcpuid, vcpuid: {}", vcpuid);
        if let Some(vcpu) = self.vcpus.get_vcpu(vcpuid) {
            return Some(vcpu.pcpu_id);
        };
        None
    }

    fn pcpuid_to_vcpuid(&self, pcpuid: usize) -> Option<usize> {
        for vcpuid in 0..self.vcpus.length {
            if let Some(vcpu) = self.vcpus.get_vcpu(vcpuid) {
                if vcpu.pcpu_id == pcpuid {
                    return Some(vcpuid);
                }
            }
        }
        None
    }
}
