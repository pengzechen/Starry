use hypercraft::arch::vuart::Vuart;
use super::vuart::*;
use crate::{HyperCraftHalImpl, GuestPageTable};
use hypercraft::arch::emu::{EmuContext, EmuDevs};
use hypercraft::VM;
use super::active_vm;

const VUART_REG_OFFSET_PREFIX_DR:usize = 0x0000;
const VUART_REG_OFFSET_PREFIX_FR:usize = 0x0018;
const VUART_REG_OFFSET_PREFIX_IBRD:usize = 0x0024;
const VUART_REG_OFFSET_PREFIX_FBRD:usize = 0x0028;
const VUART_REG_OFFSET_PREFIX_LCRH:usize = 0x002c;
const VUART_REG_OFFSET_PREFIX_CR:usize = 0x0030;
const VUART_REG_OFFSET_PREFIX_RIS:usize = 0x003c;
const VUART_REG_OFFSET_PREFIX_IFLS:usize = 0x0034;
const VUART_REG_OFFSET_PREFIX_IMSC:usize = 0x0038;
const VUART_REG_OFFSET_PREFIX_ICR:usize = 0x0044;

const VUART_REG_OFFSET_PREFIX_HARDCODE:usize = 0x0fe0;

pub fn emu_uart_handler(emu_dev_id: usize, emu_ctx: &EmuContext) -> bool {
    // get the 0 to 7th bit of address, because we onlyl use uart offset end in 0x48
    let offset = emu_ctx.address & 0xfff;
    // max width bit is 0b11 (0b11 Doubleword)
    if emu_ctx.width > 4 {
        return false;
    }
    let vm = active_vm();
    let vuart = vm.vuart_mut();
    match offset {
        VUART_REG_OFFSET_PREFIX_DR => {
            emu_uartdr_access(vuart, emu_ctx);
        }
        VUART_REG_OFFSET_PREFIX_FR => {
            emu_uartfr_access(vuart, emu_ctx);
        }
        VUART_REG_OFFSET_PREFIX_RIS => {
            emu_uartris_access(vuart, emu_ctx);
        }
        VUART_REG_OFFSET_PREFIX_ICR => {
            emu_uarticr_access(vuart, emu_ctx);
        }
        VUART_REG_OFFSET_PREFIX_CR | VUART_REG_OFFSET_PREFIX_IFLS
        | VUART_REG_OFFSET_PREFIX_IMSC
        | VUART_REG_OFFSET_PREFIX_FBRD | VUART_REG_OFFSET_PREFIX_LCRH
        | VUART_REG_OFFSET_PREFIX_IBRD => {
            debug!("[emu_uart_handler] offset:{:#x} address: {:#x} is write:{} width:{}, reg:{}", offset, emu_ctx.address, emu_ctx.write, emu_ctx.width, emu_ctx.reg);
            debug!("[emu_uart_handler] these registers are not support for multi vm");
        }
        _ => {
            // begin with 0xfe0 ~ 0xffc
            if offset & 0b1111_1110_0000 == VUART_REG_OFFSET_PREFIX_HARDCODE {
                emu_uarthardcode_access(vuart, emu_ctx);
            } else {
                panic!("[emu_uart_handler]offset:{:#x} address: {:#x} is write:{} width:{}, reg:{} these registers are not support for multi vm", offset, emu_ctx.address, emu_ctx.write, emu_ctx.width, emu_ctx.reg);
            }
        }
    }
    true
}

pub fn emu_uart_init(vm: &mut VM<HyperCraftHalImpl, GuestPageTable>, emu_dev_id: usize) {
    let vuart = Vuart::new(vm.vm_id);
    debug!("[emu_uart_init] this is vuart id {}", vuart.id);
    vm.set_emu_devs(emu_dev_id, EmuDevs::<HyperCraftHalImpl, GuestPageTable>::Vuart(vuart));
}