use hypercraft::arch::vuart::{Vuart, BUF_CAP};
use hypercraft::arch::emu::EmuContext;
use axhal::UART;
use crate::hv::kernel::current_cpu;

pub fn emu_uartdr_access(vuart: &mut Vuart,  emu_ctx: &EmuContext) {
    let idx = emu_ctx.reg;
    let mut val = if emu_ctx.write {
        current_cpu().get_gpr(idx)
    } else {
        0
    };
    // debug!("[emu_uartdr_access] address: {:#x} is write:{} width:{}, reg:{}", emu_ctx.address, emu_ctx.write, emu_ctx.width, emu_ctx.reg);
    //debug!("[emu_uartdr_access] write val: {:#x} ", val);
    // write to dr (output)
    if emu_ctx.write {
        let ch: u8 = (val & 0b1111_1111) as u8;
        if ch == ('\n' as u8) {
            let mut result = [0u8; BUF_CAP + 1];
            let mut ptr = 0;
            while !vuart.transmit_fifo.is_empty() {
                result[ptr] = vuart.transmit_fifo.pop();
                ptr += 1;
            }
            match core::str::from_utf8(&result) {
                Ok(s) =>  {
                    if vuart.id == 0 {
                        info!("vm {} output:{}", vuart.id, s);
                    } else if vuart.id == 1 {
                        warn!("vm {} output:{}", vuart.id, s);
                    }
                }
                Err(e) => warn!("Failed to convert result to string: {}", e),
            } 
        }else {
            vuart.transmit_fifo.push(ch);
        }
    }
    // read from dr (input)
    else {
        info!("[emu_uartdr_access] read to do!!!!!!!!!!");
        if vuart.receive_fifo.is_empty() {
            debug!("[emu_uartdr_access] receive_fifo is empty");
            return;
        } else {
            let ch = vuart.receive_fifo.pop() as usize;
            if let Some(dr_val) = axhal::console::getchar() {
                debug!("[emu_uartdr_access] dr_val: {:#x} ch: {:#x}", dr_val, ch);
                current_cpu().set_gpr(idx, dr_val as usize);
            } else {
                debug!("[emu_uartdr_access] dr_val: {:#x} ch: {:#x}", 0, ch);
                current_cpu().set_gpr(idx, ch);
            
            }
        }
    }
    //debug!("[emu_uartdr_access] finish");
}

pub fn emu_uartfr_access(vuart: &Vuart,  emu_ctx: &EmuContext) {
    //debug!("[emu_uartfr_access] vuart id {}", vuart.id);
    //debug!("[emu_uartfr_access] address: {:#x} is write:{} width:{}, reg:{}", emu_ctx.address, emu_ctx.write, emu_ctx.width, emu_ctx.reg);
    let mut val: usize = 0;
    //debug!("!!!!!!!!!!!!!!!!!!!!!!!!![new vuart] receive_fifio empty:{}", vuart.receive_fifo.is_empty());
    if vuart.receive_fifo.is_empty() {
        val |= 1 << 4;
    } if vuart.transmit_fifo.is_empty()  {
        val |= 1 << 7;
    };
    // debug!("[emu_uartfr_access] write val: {:#x} ", val);
    let idx = emu_ctx.reg;
    current_cpu().set_gpr(idx, val as usize);
}

pub fn emu_uartris_access(vuart: &Vuart,  emu_ctx: &EmuContext) {
    // read only register
    debug!("[emu_uartris_access] address: {:#x} is write:{} width:{}, reg:{}", emu_ctx.address, emu_ctx.write, emu_ctx.width, emu_ctx.reg);
    // let val = UART.lock().get_ris();
    let val = 1<<4;
    let idx = emu_ctx.reg;
    debug!("[emu_uartris_access] read val: {:#x} idx :{}", val, idx);
    current_cpu().set_gpr(idx, val as usize);
}

pub fn emu_uarticr_access(vuart: &mut Vuart, emu_ctx: &EmuContext) {
    //debug!("[emu_uarticr_access] address: {:#x} is write:{} width:{}, reg:{}", emu_ctx.address, emu_ctx.write, emu_ctx.width, emu_ctx.reg);
    let idx = emu_ctx.reg;
    let val = if emu_ctx.write {
        current_cpu().get_gpr(idx)
    } else {
        panic!("write only register");
    };
    //debug!("[emu_uarticr_access] write val: {:#x} ", val);
    // write to icr
    if emu_ctx.write {
        vuart.icr = val as u32;
    }
    //debug!("[emu_uarticr_access] end of emu_uarticr_access");
}

pub fn emu_uarthardcode_access(vuart: &mut Vuart, emu_ctx: &EmuContext) {
    let offset = emu_ctx.address & 0xfff;
    let val = if offset == 0x0fe0 {
        UART.lock().get_periphid0()
    }else if offset == 0x0fe4 {
        UART.lock().get_periphid1()
    }else if offset == 0x0fe8 {
        UART.lock().get_periphid2()
    }else if offset == 0x0fec {
        UART.lock().get_periphid3()
    }else if offset == 0x0ff0 {
        UART.lock().get_pcellid0()
    }else if offset == 0x0ff4 {
        UART.lock().get_pcellid1()
    }else if offset == 0x0ff8 {
        UART.lock().get_pcellid2()
    }else if offset == 0x0ffc {
        UART.lock().get_pcellid3()
    }else {
        panic!("[emu_uarthardcode_access]: offset is not in range");
    };
    debug!("[emu_uarthardcode_access] offset:{:#x} is write:{} reg:{} val:{:#x}", offset, emu_ctx.write, emu_ctx.reg, val);
    let idx = emu_ctx.reg;
    current_cpu().set_gpr(idx, val as usize);
}

/* 
pub fn emu_uarticr_access(vuart: &mut Vuart, emu_ctx: &EmuContext) {
    debug!("[emu_uarticr_access] address: {:#x} is write:{} width:{}, reg:{}", emu_ctx.address, emu_ctx.write, emu_ctx.width, emu_ctx.reg);
    let idx = emu_ctx.reg;
    let val = if emu_ctx.write {
        current_cpu().get_gpr(idx)
    } else {
        panic!("write only register");
    };
    debug!("[emu_uarticr_access] write val: {:#x} ", val);
    // write to icr
    if emu_ctx.write {
        UART.lock().set_icr(val as u32);
    }
    debug!("[emu_uarticr_access] end of emu_uarticr_access");
}
*/