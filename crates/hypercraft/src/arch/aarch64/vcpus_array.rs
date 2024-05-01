use alloc::vec::Vec;

use crate::arch::{VCpu, VM};
use crate::{GuestPageTableTrait, HyperCraftHal, HyperError, HyperResult,};


/// The maximum number of CPUs we can support.
pub const MAX_CPUS: usize = 8;

pub const VM_CPUS_MAX: usize = MAX_CPUS;

/// The set of vCPUs in a VM.
#[derive(Default, Clone, Debug)]
pub struct VcpusArray<H: HyperCraftHal> {
    inner: Vec<Option<VCpu<H>>>,
    marker: core::marker::PhantomData<H>,
    /// The number of vCPUs in the set.
    pub length: usize,
}

impl<H: HyperCraftHal> VcpusArray<H> {
    /// Creates a new vCPU tracking structure.
    pub fn new() -> Self {
        let mut inner = Vec::with_capacity(VM_CPUS_MAX);
        for _ in 0..VM_CPUS_MAX {
            inner.push(None);
        }
        Self {
            inner: inner,
            marker: core::marker::PhantomData,
            length: 0,
        }
    }

    /// Adds the given vCPU to the set of vCPUs.
    pub fn add_vcpu(&mut self, vcpu: VCpu<H>) -> HyperResult<()> {
        if self.length >= VM_CPUS_MAX {
            return Err(HyperError::NotFound);
        }
        let vcpu_id = vcpu.vcpu_id();
        self.inner[vcpu_id] = Some(vcpu);
        self.length += 1;
        Ok(())
    }
    
    /// Return true if vcpu exist
    pub fn is_vcpu_exist(&self, vcpu_id: usize) -> bool {
        if vcpu_id >= VM_CPUS_MAX {
            return false;
        }
        match & self.inner[vcpu_id] {
            Some(_) => true,
            None => false,
        }
    }

    /// Returns a reference to the vCPU with `vcpu_id` if it exists.
    pub fn get_vcpu(&self, vcpu_id: usize) -> Option<& VCpu<H>> {
        if vcpu_id >= VM_CPUS_MAX {
            return None;
        }
        match & self.inner[vcpu_id] {
            Some(vcpu) => Some(vcpu),
            None => None,
        }
    }

    /// Returns a mut reference to the vCPU with `vcpu_id` if it exists.
    pub fn get_vcpu_mut(&mut self, vcpu_id: usize) -> Option<&mut VCpu<H>> {
        if vcpu_id >= VM_CPUS_MAX {
            return None;
        }
        match &mut self.inner[vcpu_id] {
            Some(vcpu) => Some(vcpu),
            None => None,
        }
    }
}

// Safety: Each VCpu is wrapped with a Mutex to provide safe concurrent access to VCpu.
unsafe impl<H: HyperCraftHal> Sync for VcpusArray<H> {}
unsafe impl<H: HyperCraftHal> Send for VcpusArray<H> {}
