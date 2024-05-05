// use crate::platform::mem::init_mmu;
use aarch64_cpu::{asm, asm::barrier, registers::*};
use axconfig::TASK_STACK_SIZE;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

#[link_section = ".bss.stack"]
static mut BOOT_STACK: [u8; TASK_STACK_SIZE] = [0; TASK_STACK_SIZE];

unsafe fn switch_to_el1() {
    SPSel.write(SPSel::SP::ELx);
    SP_EL0.set(0);
    let current_el = CurrentEL.read(CurrentEL::EL);
    if current_el >= 2 {
        if current_el == 3 {
            // Set EL2 to 64bit and enable the HVC instruction.
            SCR_EL3.write(
                SCR_EL3::NS::NonSecure + SCR_EL3::HCE::HvcEnabled + SCR_EL3::RW::NextELIsAarch64,
            );
            // Set the return address and exception level.
            SPSR_EL3.write(
                SPSR_EL3::M::EL1h
                    + SPSR_EL3::D::Masked
                    + SPSR_EL3::A::Masked
                    + SPSR_EL3::I::Masked
                    + SPSR_EL3::F::Masked,
            );
            ELR_EL3.set(LR.get());
        }
        // Disable EL1 timer traps and the timer offset.
        CNTHCTL_EL2.modify(CNTHCTL_EL2::EL1PCEN::SET + CNTHCTL_EL2::EL1PCTEN::SET);
        CNTVOFF_EL2.set(0);
        // Set EL1 to 64bit.
        HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);
        // Set the return address and exception level.
        SPSR_EL2.write(
            SPSR_EL2::M::EL1h
                + SPSR_EL2::D::Masked
                + SPSR_EL2::A::Masked
                + SPSR_EL2::I::Masked
                + SPSR_EL2::F::Masked,
        );
        core::arch::asm!(
            "
            mov     x8, sp
            msr     sp_el1, x8"
        );
        ELR_EL2.set(LR.get());
        asm::eret();
    }
}

unsafe fn enable_fp() {
    if cfg!(feature = "fp_simd") {
        CPACR_EL1.write(CPACR_EL1::FPEN::TrapNothing);
        barrier::isb(barrier::SY);
    }
}


use page_table_entry::aarch64::A64PTE;
use memory_addr::PhysAddr;

use crate::platform::mem::BOOT_PT_L0;
use crate::platform::mem::BOOT_PT_L1;

unsafe fn init_boot_page_table() {
    crate::platform::mem::init_boot_page_table(&mut BOOT_PT_L0, &mut BOOT_PT_L1);
}

extern "C" {
    fn exception_vector_base_el2();
}

unsafe fn cache_invalidate(cache_level: usize) {
    core::arch::asm!(
        r#"
        msr csselr_el1, {0}
        mrs x4, ccsidr_el1 // read cache size id.
        and x1, x4, #0x7
        add x1, x1, #0x4 // x1 = cache line size.
        ldr x3, =0x7fff
        and x2, x3, x4, lsr #13 // x2 = cache set number – 1.
        ldr x3, =0x3ff
        and x3, x3, x4, lsr #3 // x3 = cache associativity number – 1.
        clz w4, w3 // x4 = way position in the cisw instruction.
        mov x5, #0 // x5 = way counter way_loop.
    // way_loop:
    1:
        mov x6, #0 // x6 = set counter set_loop.
    // set_loop:
    2:
        lsl x7, x5, x4
        orr x7, {0}, x7 // set way.
        lsl x8, x6, x1
        orr x7, x7, x8 // set set.
        dc cisw, x7 // clean and invalidate cache line.
        add x6, x6, #1 // increment set counter.
        cmp x6, x2 // last set reached yet?
        ble 2b // if not, iterate set_loop,
        add x5, x5, #1 // else, next way.
        cmp x5, x3 // last way reached yet?
        ble 1b // if not, iterate way_loop
        "#,
        in(reg) cache_level,
        options(nostack)
    );
}

unsafe fn init_mmu_el2() {
    /* 
    MAIR_EL2.write(
        MAIR_EL2::Attr0_Device::nonGathering_nonReordering_noEarlyWriteAck
            + MAIR_EL2::Attr1_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc
            + MAIR_EL2::Attr1_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc
            + MAIR_EL2::Attr2_Normal_Outer::NonCacheable
            + MAIR_EL2::Attr2_Normal_Inner::NonCacheable,
    );
    TCR_EL2.write(
        TCR_EL2::PS::Bits_40
            + TCR_EL2::SH0::Inner
            + TCR_EL2::TG0::KiB_4
            + TCR_EL2::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL2::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL2::T0SZ.val(16),
    );
    */
    
    // Set EL1 to 64bit.
    HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);

    // Device-nGnRE memory
    let attr0 = MAIR_EL2::Attr0_Device::nonGathering_nonReordering_EarlyWriteAck;
    // Normal memory
    let attr1 = MAIR_EL2::Attr1_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc
        + MAIR_EL2::Attr1_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc;
    MAIR_EL2.write(attr0 + attr1); // 0xff_04

     // Enable TTBR0 and TTBR1 walks, page size = 4K, vaddr size = 48 bits, paddr size = 40 bits.
    let tcr_flags0 = TCR_EL2::TG0::KiB_4
        + TCR_EL2::SH0::Inner
         + TCR_EL2::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
         + TCR_EL2::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
         + TCR_EL2::T0SZ.val(16);
    TCR_EL2.write(TCR_EL2::PS::Bits_40 + tcr_flags0);
    barrier::isb(barrier::SY);

    let root_paddr = PhysAddr::from(BOOT_PT_L0.as_ptr() as usize).as_usize() as _;
    TTBR0_EL2.set(root_paddr);
    // #[macro_use]
    // hypercraft::msr!(TTBR1_EL2, root_paddr);

    // Flush the entire TLB
    crate::arch::flush_tlb(None);

    // Enable the MMU and turn on I-cache and D-cache
    // SCTLR_EL2.set(0x30c51835);
    SCTLR_EL2.modify(SCTLR_EL2::M::Enable + SCTLR_EL2::C::Cacheable + SCTLR_EL2::I::Cacheable);
    barrier::isb(barrier::SY);
}

unsafe fn switch_to_el2() {
    SPSel.write(SPSel::SP::ELx);
    let current_el = CurrentEL.read(CurrentEL::EL);

    if current_el == 3 {
        SCR_EL3.write(
            SCR_EL3::NS::NonSecure + SCR_EL3::HCE::HvcEnabled + SCR_EL3::RW::NextELIsAarch64,
        );
        SPSR_EL3.write(
            SPSR_EL3::M::EL2h
                + SPSR_EL3::D::Masked
                + SPSR_EL3::A::Masked
                + SPSR_EL3::I::Masked
                + SPSR_EL3::F::Masked,
        );
        ELR_EL3.set(LR.get());
        SP_EL1.set(BOOT_STACK.as_ptr_range().end as u64);
        // This should be SP_EL2. To
        asm::eret();
    }
}

/// The earliest entry point for the primary CPU.
#[naked]
#[no_mangle]
#[link_section = ".text.boot"]
unsafe extern "C" fn _start() -> ! {
    // PC = 0x8_0000
    // X0 = dtb
    /*
    #[cfg(not(feature = "hv"))]
    core::arch::asm!("
        mrs     x19, mpidr_el1
        and     x19, x19, #0xffffff     // get current CPU id
        mov     x20, x0                 // save DTB pointer

        adrp    x8, {boot_stack}        // setup boot stack
        add     x8, x8, {boot_stack_size}
        mov     sp, x8

        bl      {switch_to_el1}         // switch to EL1

        adrp    x0, {start}                // kernel image phys addr
        bl      {idmap_kernel}

        bl      {init_mmu}              // setup MMU
        bl      {enable_fp}             // enable fp/neon

        mov     x8, {phys_virt_offset}  // set SP to the high address
        add     sp, sp, x8

        mov     x0, x19                 // call rust_entry(cpu_id, dtb)
        mov     x1, x20
        ldr     x8, ={entry}
        blr     x8
        b      .",
        switch_to_el1 = sym switch_to_el1,
        init_mmu = sym init_mmu,
        enable_fp = sym enable_fp,
        boot_stack = sym BOOT_STACK,
        start = sym _start,
        idmap_kernel = sym crate::platform::mem::idmap_kernel,
        boot_stack_size = const TASK_STACK_SIZE,
        phys_virt_offset = const axconfig::PHYS_VIRT_OFFSET,
        entry = sym crate::platform::rust_entry,
        options(noreturn),
    );
    */
    // set vbar_el2 for hypervisor.
    // #[cfg(feature = "hv")]
    core::arch::asm!("
        // disable cache and MMU
        mrs x1, sctlr_el2
        bic x1, x1, #0xf
        msr sctlr_el2, x1

        // cache_invalidate(0): clear dl1$
        mov x0, #0
        bl  {cache_invalidate}
        mov x0, #2
        bl  {cache_invalidate}

        ldr x8, ={exception_vector_base_el2}    // setup vbar_el2 for hypervisor
        msr vbar_el2, x8

        mrs     x19, mpidr_el1
        and     x19, x19, #0xffffff     // get current CPU id
        mov     x20, x0                 // save DTB pointer
        adrp    x8, {boot_stack}        // setup boot stack
        add     x8, x8, {boot_stack_size}
        mov     sp, x8

        bl      {init_boot_page_table}
        bl      {init_mmu_el2}
        bl      {switch_to_el2}         // switch to EL1
        bl      {enable_fp}             // enable fp/neon

        mov     x8, {phys_virt_offset}  // set SP to the high address
        add     sp, sp, x8

        mov     x0, x19                 // call rust_entry(cpu_id, dtb)
        mov     x1, x20
        ldr     x8, ={entry}
        blr     x8
        b      .",
        cache_invalidate = sym cache_invalidate,
        exception_vector_base_el2 = sym exception_vector_base_el2,
        init_boot_page_table = sym init_boot_page_table,
        init_mmu_el2 = sym init_mmu_el2,
        switch_to_el2 = sym switch_to_el2,
        enable_fp = sym enable_fp,
        boot_stack = sym BOOT_STACK,
        boot_stack_size = const TASK_STACK_SIZE,
        phys_virt_offset = const axconfig::PHYS_VIRT_OFFSET,
        entry = sym crate::platform::rust_entry,
        options(noreturn),
    );
}

/// The earliest entry point for the secondary CPUs.
#[cfg(feature = "smp")]
#[naked]
#[no_mangle]
#[link_section = ".text.boot"]
unsafe extern "C" fn _start_secondary() -> ! {
    core::arch::asm!("
        mrs     x19, mpidr_el1
        and     x19, x19, #0xffffff     // get current CPU id

        mov     sp, x0
        bl      {switch_to_el1}
        bl      {init_mmu}
        bl      {enable_fp}

        mov     x8, {phys_virt_offset}  // set SP to the high address
        add     sp, sp, x8

        mov     x0, x19                 // call rust_entry_secondary(cpu_id)
        ldr     x8, ={entry}
        blr     x8
        b      .",
        switch_to_el1 = sym switch_to_el1,
        init_mmu = sym init_mmu,
        enable_fp = sym enable_fp,
        phys_virt_offset = const axconfig::PHYS_VIRT_OFFSET,
        entry = sym crate::platform::rust_entry_secondary,
        options(noreturn),
    )
}
