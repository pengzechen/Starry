mod device_emu;

use hypercraft::{
    VmxExitReason, 
    VCpu as HVCpu, 
    HyperResult, 
    HyperError, 
    VmxExitInfo
};

use device_emu::VirtLocalApic;

type VCpu = HVCpu<super::HyperCraftHalImpl>;

const VM_EXIT_INSTR_LEN_CPUID: u8 = 2;
const VM_EXIT_INSTR_LEN_RDMSR: u8 = 2;
const VM_EXIT_INSTR_LEN_WRMSR: u8 = 2;
const VM_EXIT_INSTR_LEN_VMCALL: u8 = 3;

fn handle_external_interrupt(vcpu: &mut VCpu) -> HyperResult {
    #[cfg(feature = "irq")]
    {
        let int_info = vcpu.interrupt_exit_info()?;
        trace!("VM-exit: external interrupt: {:#x?}", int_info);
        assert!(int_info.valid);
        axhal::irq::dispatch_irq(int_info.vector as usize);
        Ok(())
    }
    #[cfg(not(feature = "irq"))] {
        panic!("cannot handle EXTERNAL_INTERRUPT vmexit because \"irq\" is not enabled")
    }
}

fn handle_cpuid(vcpu: &mut VCpu) -> HyperResult {
    use raw_cpuid::{cpuid, CpuIdResult};

    const LEAF_FEATURE_INFO: u32 = 0x1;
    const LEAF_HYPERVISOR_INFO: u32 = 0x4000_0000;
    const LEAF_HYPERVISOR_FEATURE: u32 = 0x4000_0001;
    const VENDOR_STR: &[u8; 12] = b"RVMRVMRVMRVM";
    let vendor_regs = unsafe { &*(VENDOR_STR.as_ptr() as *const [u32; 3]) };

    let regs = vcpu.regs_mut();
    let function = regs.rax as u32;
    let res = match function {
        LEAF_FEATURE_INFO => {
            const FEATURE_VMX: u32 = 1 << 5;
            const FEATURE_HYPERVISOR: u32 = 1 << 31;
            let mut res = cpuid!(regs.rax, regs.rcx);
            res.ecx &= !FEATURE_VMX;
            res.ecx |= FEATURE_HYPERVISOR;
            res
        }
        LEAF_HYPERVISOR_INFO => CpuIdResult {
            eax: LEAF_HYPERVISOR_FEATURE,
            ebx: vendor_regs[0],
            ecx: vendor_regs[1],
            edx: vendor_regs[2],
        },
        LEAF_HYPERVISOR_FEATURE => CpuIdResult {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
        },
        _ => cpuid!(regs.rax, regs.rcx),
    };

    debug!(
        "VM exit: CPUID({:#x}, {:#x}): {:?}",
        regs.rax, regs.rcx, res
    );
    regs.rax = res.eax as _;
    regs.rbx = res.ebx as _;
    regs.rcx = res.ecx as _;
    regs.rdx = res.edx as _;
    vcpu.advance_rip(VM_EXIT_INSTR_LEN_CPUID)?;
    Ok(())
}

fn handle_io_instruction(vcpu: &mut VCpu, exit_info: &VmxExitInfo) -> HyperResult {
    let io_info = vcpu.io_exit_info()?;
    trace!(
        "VM exit: I/O instruction @ {:#x}: {:#x?}",
        exit_info.guest_rip,
        io_info,
    );
    if io_info.is_string {
        error!("INS/OUTS instructions are not supported!");
        return Err(HyperError::NotSupported);
    }
    if io_info.is_repeat {
        error!("REP prefixed I/O instructions are not supported!");
        return Err(HyperError::NotSupported);
    }

    if let Some(dev) = device_emu::all_virt_devices().find_port_io_device(io_info.port) {
        if io_info.is_in {
            let value = dev.read(io_info.port, io_info.access_size)?;
            let rax = &mut vcpu.regs_mut().rax;
            // SDM Vol. 1, Section 3.4.1.1:
            // * 32-bit operands generate a 32-bit result, zero-extended to a 64-bit result in the
            //   destination general-purpose register.
            // * 8-bit and 16-bit operands generate an 8-bit or 16-bit result. The upper 56 bits or
            //   48 bits (respectively) of the destination general-purpose register are not modified
            //   by the operation.
            match io_info.access_size {
                1 => *rax = (*rax & !0xff) | (value & 0xff) as u64,
                2 => *rax = (*rax & !0xffff) | (value & 0xffff) as u64,
                4 => *rax = value as u64,
                _ => unreachable!(),
            }
        } else {
            let rax = vcpu.regs().rax;
            let value = match io_info.access_size {
                1 => rax & 0xff,
                2 => rax & 0xffff,
                4 => rax,
                _ => unreachable!(),
            } as u32;
            dev.write(io_info.port, io_info.access_size, value)?;
        }
    } else {
        panic!(
            "Unsupported I/O port {:#x} access: {:#x?}",
            io_info.port, io_info
        )
    }
    vcpu.advance_rip(exit_info.exit_instruction_length as _)?;
    Ok(())
}

fn handle_msr_read(vcpu: &mut VCpu) -> HyperResult {
    let msr = vcpu.regs().rcx as u32;

    use x86::msr::*;
    let res = if msr == IA32_APIC_BASE {
        let mut apic_base = unsafe { rdmsr(IA32_APIC_BASE) };
        apic_base |= 1 << 11 | 1 << 10; // enable xAPIC and x2APIC
        Ok(apic_base)
    } else if VirtLocalApic::msr_range().contains(&msr) {
        VirtLocalApic::rdmsr(vcpu, msr)
    } else {
        Err(HyperError::NotSupported)
    };

    if let Ok(value) = res {
        trace!("VM exit: RDMSR({:#x}) -> {:#x}", msr, value);
        vcpu.regs_mut().rax = value & 0xffff_ffff;
        vcpu.regs_mut().rdx = value >> 32;
    } else {
        panic!("Failed to handle RDMSR({:#x}): {:?}", msr, res);
    }
    vcpu.advance_rip(VM_EXIT_INSTR_LEN_RDMSR)?;
    Ok(())
}

fn handle_msr_write(vcpu: &mut VCpu) -> HyperResult {
    let msr = vcpu.regs().rcx as u32;
    let value = (vcpu.regs().rax & 0xffff_ffff) | (vcpu.regs().rdx << 32);
    trace!("VM exit: WRMSR({:#x}) <- {:#x}", msr, value);

    use x86::msr::*;
    let res = if msr == IA32_APIC_BASE {
        Ok(()) // ignore
    } else if VirtLocalApic::msr_range().contains(&msr) {
        VirtLocalApic::wrmsr(vcpu, msr, value)
    } else {
        Err(HyperError::NotSupported)
    };

    if res.is_err() {
        panic!(
            "Failed to handle WRMSR({:#x}) <- {:#x}: {:?}",
            msr, value, res
        );
    }
    vcpu.advance_rip(VM_EXIT_INSTR_LEN_WRMSR)?;
    Ok(())
}

pub fn vmexit_handler(vcpu: &mut VCpu) -> HyperResult {
    let exit_info = vcpu.exit_info()?;
    
    match exit_info.exit_reason {
        VmxExitReason::EXTERNAL_INTERRUPT => handle_external_interrupt(vcpu),
        VmxExitReason::CPUID => handle_cpuid(vcpu),
        VmxExitReason::IO_INSTRUCTION => handle_io_instruction(vcpu, &exit_info),
        VmxExitReason::MSR_READ => handle_msr_read(vcpu),
        VmxExitReason::MSR_WRITE => handle_msr_write(vcpu),
        _ => panic!("vmexit reason not supported {:?}:\n{:?}", exit_info.exit_reason, vcpu)
    }
}