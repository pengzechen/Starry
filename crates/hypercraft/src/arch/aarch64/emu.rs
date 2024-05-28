use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{Display, Formatter};
use spin::Mutex;


#[cfg(not(feature = "gic_v3"))]
use super::vgic::Vgic;  // temp use
#[cfg(feature = "gic_v3")]
use super::vgicv3::Vgic;

use super::vuart::Vuart;
use crate::{HyperCraftHal, GuestPageTableTrait};

/// Emulated device
#[derive(Clone)]
pub enum EmuDevs<H: HyperCraftHal, G:GuestPageTableTrait> {
    /// Virtual gic
    Vgic(Arc<Vgic<H, G>>),
    /// Virtual uart
    Vuart(Vuart),
    /* 
    VirtioBlk(VirtioMmio),
    VirtioNet(VirtioMmio),
    VirtioConsole(VirtioMmio),
    */
    /// Nothing
    None,
}

/// Emulated device context
#[derive(Debug)]
pub struct EmuContext {
    /// fault address
    pub address: usize,
    /// instruction width
    pub width: usize,
    /// write or read
    pub write: bool,
    /// Data item whether should be sign-extended.
    pub sign_ext: bool,
    /// target or source register idx
    pub reg: usize,
    /// target or source register width
    pub reg_width: usize,
}

/// Emulated device entry
pub struct EmuDevEntry {
    /// emulated device type
    pub emu_type: EmuDeviceType,
    /// vm id
    pub vm_id: usize,
    /// device id
    pub id: usize,
    /// device address
    pub ipa: usize,
    /// device address space size
    pub size: usize,
    /// device handler
    pub handler: EmuDevHandler,
}

/// Emulated device type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EmuDeviceType {
    /// console
    EmuDeviceTConsole = 0,
    /// GIC (interrupt controller)
    EmuDeviceTGicd = 1,
    /// partial passthrough interrupt controller
    EmuDeviceTGPPT = 2,
    /// virtio block
    EmuDeviceTVirtioBlk = 3,
    /// virtio net
    EmuDeviceTVirtioNet = 4,
    /// virtio console
    EmuDeviceTVirtioConsole = 5,
    /// IOMMU
    EmuDeviceTIOMMU = 6,
}

impl Display for EmuDeviceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            EmuDeviceType::EmuDeviceTGicd => write!(f, "interrupt controller"),
            EmuDeviceType::EmuDeviceTConsole => write!(f, "console"),
            EmuDeviceType::EmuDeviceTGPPT => write!(f, "partial passthrough interrupt controller"),
            EmuDeviceType::EmuDeviceTVirtioBlk => write!(f, "virtio block"),
            EmuDeviceType::EmuDeviceTVirtioNet => write!(f, "virtio net"),
            EmuDeviceType::EmuDeviceTVirtioConsole => write!(f, "virtio console"),
            EmuDeviceType::EmuDeviceTIOMMU => write!(f, "IOMMU"),
        }
    }
}

impl EmuDeviceType {
    /// Removable
    pub fn removable(&self) -> bool {
        match *self {
            EmuDeviceType::EmuDeviceTGicd
            | EmuDeviceType::EmuDeviceTGPPT
            | EmuDeviceType::EmuDeviceTVirtioBlk
            | EmuDeviceType::EmuDeviceTVirtioNet
            | EmuDeviceType::EmuDeviceTVirtioConsole => true,
            _ => false,
        }
    }
}

impl EmuDeviceType {
    /// Convert from usize
    pub fn from_usize(value: usize) -> EmuDeviceType {
        match value {
            0 => EmuDeviceType::EmuDeviceTConsole,
            1 => EmuDeviceType::EmuDeviceTGicd,
            2 => EmuDeviceType::EmuDeviceTGPPT,
            3 => EmuDeviceType::EmuDeviceTVirtioBlk,
            4 => EmuDeviceType::EmuDeviceTVirtioNet,
            5 => EmuDeviceType::EmuDeviceTVirtioConsole,
            6 => EmuDeviceType::EmuDeviceTIOMMU,
            _ => panic!("Unknown  EmuDeviceType value: {}", value),
        }
    }
}

/// Emulated device handler
pub type EmuDevHandler = fn(usize, &EmuContext) -> bool;
