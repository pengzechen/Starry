use axhal::mem::{PhysAddr, VirtAddr};

use hypercraft::{GuestPageTableTrait, GuestPhysAddr, HyperError, HyperResult, NestedPageTable};

use page_table_entry::MappingFlags;

pub type GuestPagingIfImpl = axhal::paging::PagingIfImpl;

/// Guest Page Table struct\
#[derive(Clone)]
pub struct GuestPageTable(NestedPageTable<GuestPagingIfImpl>);

impl GuestPageTableTrait for GuestPageTable {
    fn new() -> HyperResult<Self> {
        
        {
            let agpt = NestedPageTable::<GuestPagingIfImpl>::try_new().unwrap();
            if usize::from(agpt.root_paddr()) & (1<<12) != 0 {
                let nextapt = NestedPageTable::<GuestPagingIfImpl>::try_new().map_err(|_| HyperError::NoMemory)?;
                {
                    let _ = agpt; // drop agpt
                }
                Ok(GuestPageTable(nextapt))
            }else {
                Ok(GuestPageTable(agpt))
            }
        }
    }

    fn map(
        &mut self,
        gpa: GuestPhysAddr,
        hpa: hypercraft::HostPhysAddr,
        flags: MappingFlags,
    ) -> HyperResult<()> {
        
        self.0
            .map(
                VirtAddr::from(gpa),
                PhysAddr::from(hpa),
                page_table::PageSize::Size4K,
                flags,
            )
            .map_err(|paging_err| {
                error!("paging error: {:?}", paging_err);
                HyperError::Internal
            })?;
        Ok(())
        
    }

    fn map_region(
        &mut self,
        gpa: GuestPhysAddr,
        hpa: hypercraft::HostPhysAddr,
        size: usize,
        flags: MappingFlags,
    ) -> HyperResult<()> {
        
        {
            self.0
                .map_region(VirtAddr::from(gpa), PhysAddr::from(hpa), size, flags, true)
                .map_err(|err| {
                    error!("paging error: {:?}", err);
                    HyperError::Internal
                })?;
            Ok(())
        }
    }

    fn unmap(&mut self, gpa: GuestPhysAddr) -> HyperResult<()> {
        #[cfg(any(target_arch = "riscv64", target_arch = "x86_64", target_arch = "aarch64"))]
        {
            let (_, _) = self.0.unmap(VirtAddr::from(gpa)).map_err(|paging_err| {
                error!("paging error: {:?}", paging_err);
                return HyperError::Internal;
            })?;
            Ok(())
        }
        #[cfg(not(any(target_arch = "riscv64", target_arch = "x86_64", target_arch = "aarch64")))]
        {
            todo!()
        }
    }

    fn translate(&self, gpa: GuestPhysAddr) -> HyperResult<hypercraft::HostPhysAddr> {
        #[cfg(any(target_arch = "riscv64", target_arch = "x86_64", target_arch = "aarch64"))]
        {
            let (addr, _, _) = self.0.query(VirtAddr::from(gpa)).map_err(|paging_err| {
                error!("paging error: {:?}", paging_err);
                HyperError::Internal
            })?;
            Ok(addr.into())
        }
        #[cfg(not(any(target_arch = "riscv64", target_arch = "x86_64", target_arch = "aarch64")))]
        {
            todo!()
        }
    }

    fn token(&self) -> usize {
        
        usize::from(self.0.root_paddr())  // need to lrs 1 bit for CnP??

    }
}

impl GuestPageTable {
    pub fn root_paddr(&self) -> PhysAddr {
        self.0.root_paddr()
    }
}
