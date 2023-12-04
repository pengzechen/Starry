
       d8888                            .d88888b.   .d8888b.
      d88888                           d88P" "Y88b d88P  Y88b
     d88P888                           888     888 Y88b.
    d88P 888 888d888  .d8888b  .d88b.  888     888  "Y888b.
   d88P  888 888P"   d88P"    d8P  Y8b 888     888     "Y88b.
  d88P   888 888     888      88888888 888     888       "888
 d8888888888 888     Y88b.    Y8b.     Y88b. .d88P Y88b  d88P
d88P     888 888      "Y8888P  "Y8888   "Y88888P"   "Y8888P"

arch = aarch64
platform = qemu-virt-aarch64
smp = 2
build_mode = release
log_level = debug

[37m[  0.002136 axruntime:148] [32mLogging is enabled.[m
[m[37m[  0.002928 axruntime:149] [32mPrimary CPU 0 started, dtb = 0x2.[m
[m[37m[  0.003124 axruntime:154] [32mFound physcial memory regions:[m
[m[37m[  0.003326 axruntime:156] [32m  [PA:0x40080000, PA:0x40099000) .text (READ | EXECUTE | RESERVED)[m
[m[37m[  0.003591 axruntime:156] [32m  [PA:0x40099000, PA:0x4009f000) .rodata (READ | RESERVED)[m
[m[37m[  0.003716 axruntime:156] [32m  [PA:0x4009f000, PA:0x400a2000) .data (READ | WRITE | RESERVED)[m
[m[37m[  0.003836 axruntime:156] [32m  [PA:0x400a2000, PA:0x400a4000) .percpu (READ | WRITE | RESERVED)[m
[m[37m[  0.003947 axruntime:156] [32m  [PA:0x400a4000, PA:0x40124000) boot stack (READ | WRITE | RESERVED)[m
[m[37m[  0.004061 axruntime:156] [32m  [PA:0x40124000, PA:0x4014a000) .bss (READ | WRITE | RESERVED)[m
[m[37m[  0.004179 axruntime:156] [32m  [PA:0x9000000, PA:0x9001000) mmio (READ | WRITE | DEVICE | RESERVED)[m
[m[37m[  0.004295 axruntime:156] [32m  [PA:0x8000000, PA:0x8040000) mmio (READ | WRITE | DEVICE | RESERVED)[m
[m[37m[  0.004404 axruntime:156] [32m  [PA:0xa000000, PA:0xa004000) mmio (READ | WRITE | DEVICE | RESERVED)[m
[m[37m[  0.004514 axruntime:156] [32m  [PA:0x10000000, PA:0x3eff0000) mmio (READ | WRITE | DEVICE | RESERVED)[m
[m[37m[  0.004626 axruntime:156] [32m  [PA:0x4010000000, PA:0x4020000000) mmio (READ | WRITE | DEVICE | RESERVED)[m
[m[37m[  0.004757 axruntime:156] [32m  [PA:0x4014a000, PA:0x48000000) free memory (READ | WRITE | FREE)[m
[m[37m[  0.004897 axruntime:167] [32mInitialize global memory allocator...[m
[m[37m[  0.005204 axalloc:184] [36minitialize global allocator at: [0x4014a000, 0x48000000)[m
[m[37m[  0.006244 axruntime:180] [32mInitialize platform devices...[m
[m[37m[  0.006355 axhal::platform::aarch64_common::gic:85] [32mInitialize GICv2...[m
[m[37m[  0.006744 axhal::platform::aarch64_common::generic_timer:56] [36minit_percpu in timer[m
[m[37m[  0.006909 axhal::platform::aarch64_common::gic:59] [36min platform gic set_enable: irq_num 30, enabled true[m
[m[37m[  0.007206 axhal::platform::aarch64_common::gic:59] [36min platform gic set_enable: irq_num 33, enabled true[m
[m[37m[  0.007380 axruntime::mp:18] [36mstarting CPU 1...[m
[m[37m[  0.007496 axhal::platform::aarch64_common::psci:68] [36mStarting core 1...[m
[m[37m[  0.007636 axhal::platform::aarch64_common::psci:35] [36mthis is smc call func:0xc4000003[m
[m[37m[  0.007841 axhal::platform::aarch64_common::psci:73] [36mStarted core 1![m
[m[37m[  0.008284 axruntime::mp:35] [32mSecondary CPU 1 started.[m
[m[37m[  0.008294 axruntime:206] [32mInitialize interrupt handlers...[m
[m[37m[  0.008423 axhal::platform::aarch64_common::gic:97] [32mInitialize init_secondary GICv2...[m
[m[37m[  0.008516 axruntime:323] [36minit ipi interrupt handler[m
[m[37m[  0.008635 axhal::platform::aarch64_common::generic_timer:56] [36minit_percpu in timer[m
[m[37m[  0.008748 axhal::irq:30] [36m!!!!!!!!!!!!!!register handler for IRQ 1 success[m
[m[37m[  0.008790 axhal::platform::aarch64_common::gic:59] [36min platform gic set_enable: irq_num 30, enabled true[m
[m[37m[  0.008893 axhal::platform::aarch64_common::gic:59] [36min platform gic set_enable: irq_num 1, enabled true[m
[m[37m[  0.009046 axruntime::mp:53] [32mSecondary CPU 1 init OK.[m
[m[37m[  0.009831 axruntime:210] [32mPrimary CPU 0 init OK.[m
[mHello, hv!
[37m[  0.010027 1 arceos_hv:143] [32mHello World from cpu 1[m
[m[37m[  0.010080 0 hypercraft::arch::cpu:68] [36mpcpu_size: 0x2000[m
[m[37m[  0.010532 0 hypercraft::arch::cpu:71] [36mpcpu_pages: 0x40152000[m
[m[37m[  0.013487 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa000000, size: 0x200[m
[m[37m[  0.013624 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa000200, size: 0x200[m
[m[37m[  0.013731 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa000400, size: 0x200[m
[m[37m[  0.013839 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa000600, size: 0x200[m
[m[37m[  0.013946 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa000800, size: 0x200[m
[m[37m[  0.014052 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa000a00, size: 0x200[m
[m[37m[  0.014158 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa000c00, size: 0x200[m
[m[37m[  0.014266 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa000e00, size: 0x200[m
[m[37m[  0.014373 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa001000, size: 0x200[m
[m[37m[  0.014479 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa001200, size: 0x200[m
[m[37m[  0.014586 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa001400, size: 0x200[m
[m[37m[  0.014692 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa001600, size: 0x200[m
[m[37m[  0.014799 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa001800, size: 0x200[m
[m[37m[  0.014905 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa001a00, size: 0x200[m
[m[37m[  0.015011 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa001c00, size: 0x200[m
[m[37m[  0.015118 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa001e00, size: 0x200[m
[m[37m[  0.015225 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa002000, size: 0x200[m
[m[37m[  0.015331 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa002200, size: 0x200[m
[m[37m[  0.015437 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa002400, size: 0x200[m
[m[37m[  0.015544 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa002600, size: 0x200[m
[m[37m[  0.015651 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa002800, size: 0x200[m
[m[37m[  0.015758 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa002a00, size: 0x200[m
[m[37m[  0.015864 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa002c00, size: 0x200[m
[m[37m[  0.015971 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa002e00, size: 0x200[m
[m[37m[  0.016077 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa003000, size: 0x200[m
[m[37m[  0.016188 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa003200, size: 0x200[m
[m[37m[  0.016295 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa003400, size: 0x200[m
[m[37m[  0.016401 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa003600, size: 0x200[m
[m[37m[  0.016507 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa003800, size: 0x200[m
[m[37m[  0.016614 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa003a00, size: 0x200[m
[m[37m[  0.016721 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa003c00, size: 0x200[m
[m[37m[  0.016827 0 arceos_hv::dtb_aarch64:42] [36mvirtio mmio addr: 0xa003e00, size: 0x200[m
[m[37m[  0.017281 0 arceos_hv::dtb_aarch64:56] [36mpl011 addr: 0x9000000, size: 0x1000[m
[m[37m[  0.017565 0 arceos_hv::dtb_aarch64:67] [36mpl031 addr: 0x9010000, size: 0x1000[m
[m[37m[  0.017827 0 arceos_hv::dtb_aarch64:78] [36mpl061 addr: 0x9030000, size: 0x1000[m
[m[37m[  0.018129 0 arceos_hv::dtb_aarch64:91] [36mintc addr: 0x8000000, size: 0x10000[m
[m[37m[  0.018252 0 arceos_hv::dtb_aarch64:91] [36mintc addr: 0x8010000, size: 0x10000[m
[m[37m[  0.018579 0 arceos_hv::dtb_aarch64:103] [36mintc addr: 0x8020000, size: 0x1000[m
[m[37m[  0.018906 0 arceos_hv::dtb_aarch64:117] [36mpcie addr: 0x4010000000, size: 0x10000000[m
[m[37m[  0.019213 0 arceos_hv::dtb_aarch64:130] [36mflash addr: 0x0, size: 0x4000000[m
[m[37m[  0.019332 0 arceos_hv::dtb_aarch64:130] [36mflash addr: 0x4000000, size: 0x4000000[m
[m[37m[  0.020058 0 arceos_hv:310] [32mphysical memory: [0x70000000: 0x7f000000)[m
[m[37m[  0.020261 0 arceos_hv:324] [36mtranslate vaddr: 0x8000000014, hpa: 0x14[m
[m[37m[  0.020902 0 axalloc:90] [36mexpand heap memory: [0x4015c000, 0x4016c000)[m
[m[37m[  0.021162 0 arceos_hv:97] [36mthis is VM_ARRAY: 0x40148478[m
[m[37m[  0.021873 0 axruntime::hv::aarch64_kernel::vm_array:76] [36mcurrent pcpu id: 0 vcpu id:0[m
[m[37m[  0.021880 1 arceos_hv:155] [32mvcpu 1 init ok[m
[m[37m[  0.022087 0 hypercraft::arch::vm:50] [36mrun vcpu0[m
[m[37m[  0.022103 1 axruntime::mp:67] [36mafter init main hv[m
[m[37m[  0.022197 0 hypercraft::arch::vm:51] [33mvcpu: VmCpuRegisters { guest_trap_context_regs: Aarch64ContextFrame { gpr: [1879048192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], sp: 0, elr: 1881145344, spsr: 965 }, save_for_os_context_regs: Aarch64ContextFrame { gpr: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], sp: 0, elr: 0, spsr: 965 }, vm_system_regs: VmContext { cntvoff_el2: 0, cntp_cval_el0: 0, cntv_cval_el0: 0, cntkctl_el1: 0, cntvct_el0: 0, cntp_ctl_el0: 0, cntv_ctl_el0: 0, cntp_tval_el0: 0, cntv_tval_el0: 0, vpidr_el2: 0, vmpidr_el2: 2147483648, sp_el0: 0, sp_el1: 0, elr_el1: 0, spsr_el1: 0, sctlr_el1: 818219056, actlr_el1: 0, cpacr_el1: 0, ttbr0_el1: 0, ttbr1_el1: 0, tcr_el1: 0, esr_el1: 0, far_el1: 0, par_el1: 0, mair_el1: 0, amair_el1: 0, vbar_el1: 0, contextidr_el1: 0, tpidr_el0: 0, tpidr_el1: 0, tpidrro_el0: 0, hcr_el2: 2148007953, cptr_el2: 0, hstr_el2: 0, pmcr_el0: 0, vtcr_el2: 144728, far_el2: 0, hpfar_el2: 0, gic_state: GicState { saved_hcr: 0, saved_eisr: [0, 0], saved_elrsr: [0, 0], saved_apr: 0, saved_lr: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], saved_ctlr: 0 } } }[m
[m[37m[  0.025637 0 hypercraft::arch::vm:53] [36mvttbr_token: 0x40154000[m
[m[37m[  0.026034 0 axhal::arch::aarch64::hv::exception:62] [36m!!!!!!!!!!enter current_el_spx_irq[m
[m[37m[  0.026245 0 axhal::arch::aarch64::hv::exception:109] [36m!!!!!!!!!!enter lower_aarch64_irq id:30 src:0[m
[m[37m[  0.026510 0 axruntime::hv::aarch64_kernel::interrupt:5] [36msrc 0x0 id0x1e virtual interrupt not implement yet[m
[m[37m[  0.026758 0 axruntime::hv::aarch64_kernel::interrupt:35] [36minterrupt_handler: core 0 receive unsupported int 30[m
[m[37m[  0.091290 0 axhal::arch::aarch64::hv::exception:122] [36menter lower_aarch64_synchronous exception class:0x17[m
[m[37m[  0.091603 0 axhal::arch::aarch64::hv::guest_psci:19] [36msmc_guest_handler: fid 0x84000000, x1 0x0, x2 0x0, x3 0x0[m
[m[37m[  0.091897 0 axhal::arch::aarch64::hv::guest_psci:40] [36msmc_guest_handler: fid 0x84000000, x1 0x0, x2 0x0, x3 0x0 result: 0x10001[m
[m[37m[  0.092323 0 axhal::arch::aarch64::hv::exception:122] [36menter lower_aarch64_synchronous exception class:0x17[m
[m[37m[  0.092508 0 axhal::arch::aarch64::hv::guest_psci:19] [36msmc_guest_handler: fid 0x84000006, x1 0x0, x2 0x0, x3 0x0[m
[m[37m[  0.092695 0 axhal::arch::aarch64::hv::guest_psci:40] [36msmc_guest_handler: fid 0x84000006, x1 0x0, x2 0x0, x3 0x0 result: 0x2[m
[m[37m[  0.092948 0 axhal::arch::aarch64::hv::exception:122] [36menter lower_aarch64_synchronous exception class:0x17[m
[m[37m[  0.093128 0 axhal::arch::aarch64::hv::guest_psci:19] [36msmc_guest_handler: fid 0x8400000a, x1 0x80000000, x2 0x0, x3 0x0[m
[m[37m[  0.093356 0 axhal::arch::aarch64::hv::guest_psci:40] [36msmc_guest_handler: fid 0x8400000a, x1 0x80000000, x2 0x0, x3 0x0 result: 0xffffffffffffffff[m
[m[37m[  0.093624 0 axhal::arch::aarch64::hv::exception:122] [36menter lower_aarch64_synchronous exception class:0x17[m
[m[37m[  0.093801 0 axhal::arch::aarch64::hv::guest_psci:19] [36msmc_guest_handler: fid 0x8400000a, x1 0xc4000001, x2 0x0, x3 0x0[m
[m[37m[  0.094000 0 axhal::arch::aarch64::hv::guest_psci:40] [36msmc_guest_handler: fid 0x8400000a, x1 0xc4000001, x2 0x0, x3 0x0 result: 0xffffffffffffffff[m
[m[37m[  0.094260 0 axhal::arch::aarch64::hv::exception:122] [36menter lower_aarch64_synchronous exception class:0x17[m
[m[37m[  0.094438 0 axhal::arch::aarch64::hv::guest_psci:19] [36msmc_guest_handler: fid 0x8400000a, x1 0xc400000e, x2 0x0, x3 0x0[m
[m[37m[  0.094642 0 axhal::arch::aarch64::hv::guest_psci:40] [36msmc_guest_handler: fid 0x8400000a, x1 0xc400000e, x2 0x0, x3 0x0 result: 0xffffffffffffffff[m
[m[37m[  0.094906 0 axhal::arch::aarch64::hv::exception:122] [36menter lower_aarch64_synchronous exception class:0x17[m
[m[37m[  0.095079 0 axhal::arch::aarch64::hv::guest_psci:19] [36msmc_guest_handler: fid 0x8400000a, x1 0xc4000012, x2 0x0, x3 0x0[m
[m[37m[  0.095278 0 axhal::arch::aarch64::hv::guest_psci:40] [36msmc_guest_handler: fid 0x8400000a, x1 0xc4000012, x2 0x0, x3 0x0 result: 0xffffffffffffffff[m
[m[37m[  0.143329 0 axhal::arch::aarch64::hv::exception:109] [36m!!!!!!!!!!enter lower_aarch64_irq id:27 src:0[m
[m[37m[  0.143534 0 axruntime::hv::aarch64_kernel::interrupt:5] [36msrc 0x0 id0x1b virtual interrupt not implement yet[m
[m[37m[  0.143723 0 axruntime::hv::aarch64_kernel::interrupt:35] [36minterrupt_handler: core 0 receive unsupported int 27[m
[m[37m[  0.191170 0 axhal::arch::aarch64::hv::exception:122] [36menter lower_aarch64_synchronous exception class:0x17[m
[m[37m[  0.191369 0 axhal::arch::aarch64::hv::guest_psci:19] [36msmc_guest_handler: fid 0xc4000003, x1 0x1, x2 0x711e52c8, x3 0x0[m
[m[37m[  0.191652 0 axhal::arch::aarch64::hv::guest_psci:81] [36mthis is vcpu id 1, entry:0x711e52c8 ctx:0x0[m
[m[37m[  0.192000 0 axhal::arch::aarch64::hv::ipi:103] [36mcpu_int_list [IpiMessage { ipi_type: IpiTPower, ipi_message: Power(IpiPowerMessage { src: 0, event: PsciIpiCpuOn, entry: 1897812680, context: 0 }) }][m
[m[37m[  0.192651 0 axhal::platform::aarch64_common::gic:113] [36minterrupt_cpu_ipi_send: cpu_id 1, ipi_id 1[m
[m[37m[  0.192900 0 arm_gic::gic_v2:267] [36mset sgi!!!![m
[m[37m[  0.193046 0 axhal::arch::aarch64::hv::guest_psci:94] [36m[psci_guest_cpu_on_by_ipi] after send ipi msg!!!![m
[m[37m[  0.193234 0 axhal::arch::aarch64::hv::guest_psci:40] [36msmc_guest_handler: fid 0xc4000003, x1 0x1, x2 0x711e52c8, x3 0x0 result: 0x0[m
[mQEMU: Terminated
