use std::io::Result;

fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    
    let mut is_hv: bool = false;
    let hv_env = std::env::var("HV");
    if hv_env.is_ok() {
        let hv = hv_env.unwrap();
        if hv == "y" {
            is_hv = true;
         }
    }
    
    let platform = std::env::var("PLATFORM").unwrap_or("dummy".to_string());
    gen_linker_script(&arch, &platform, is_hv).unwrap();
}

fn gen_linker_script(arch: &str, platform: &str, is_hv: bool) -> Result<()> {
    let mut fname = format!("linker_{}.lds", platform);
    if is_hv {
        fname = format!("linker_{}_hv.lds", platform);
    }
    let output_arch = if arch == "x86_64" {
        "i386:x86-64"
    } else if arch.contains("riscv") {
        "riscv" // OUTPUT_ARCH of both riscv32/riscv64 is "riscv"
    } else {
        arch
    };
    let ld_content = std::fs::read_to_string("linker.lds.S")?;
    let ld_content = ld_content.replace("%ARCH%", output_arch);
    
    let ld_content = ld_content.replace(
        "%KERNEL_BASE%",
        &format!("{:#x}", axconfig::KERNEL_BASE_VADDR),
    );
        
    let ld_content = ld_content.replace("%SMP%", &format!("{}", axconfig::SMP));
     
    let align: &str;
    if is_hv && platform == "qemu-virt-aarch64" {
        align = r#"8K"#;
    } else {
        align = r#"4K"#;
    }
    let ld_content = ld_content.replace("%ALIGN%", align);
    /*
    let el2_link: &str;
    if is_hv {
    el2_link = r#"el2code_start = .;
    .el2code 0x10000 : AT(el2code_start) ALIGN(4096) {
        *(.el2code.test)
    }"#;
    } else {
        el2_link = r#""#;
    }
    let ld_content = ld_content.replace("%EL2CODE%", el2_link);
    */
    //#[cfg(not(feature = "hv"))]
    //

    std::fs::write(fname, ld_content)?;
    Ok(())
}
