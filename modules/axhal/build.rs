use std::io::Result;

fn has_feature(feature: &str) -> bool {
    std::env::var(format!(
        "CARGO_FEATURE_{}",
        feature.to_uppercase().replace('-', "_")
    ))
    .is_ok()
}

fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let platform = axconfig::PLATFORM;
    if platform != "dummy" {
        gen_linker_script(&arch, platform).unwrap();
    }

    println!("cargo:rustc-cfg=platform=\"{}\"", platform);
    println!("cargo:rustc-cfg=platform_family=\"{}\"", axconfig::FAMILY);
}

fn gen_linker_script(arch: &str, platform: &str) -> Result<()> {
    let mut fname = String::new();
    if has_feature("hv") {
        fname = format!("linker_{}_hv.lds", platform);
    } else {
        fname = format!("linker_{}.lds", platform);
    }
    let output_arch = if arch == "x86_64" {
        "i386:x86-64"
    } else if arch.contains("riscv") {
        "riscv" // OUTPUT_ARCH of both riscv32/riscv64 is "riscv"
    } else {
        arch
    };
    let mut ld_content = std::fs::read_to_string("linker.lds.S")?;
    ld_content = ld_content.replace("%ARCH%", output_arch);
    if has_feature("hv") {
        ld_content = ld_content.replace(
            "%KERNEL_BASE%",
            &format!("{:#x}", axconfig::HV_KERNEL_BASE_VADDR),
        );
        ld_content = ld_content.replace("%ALIGN%", &format!("8K"));
    } else {
        ld_content = ld_content.replace(
            "%KERNEL_BASE%",
            &format!("{:#x}", axconfig::KERNEL_BASE_VADDR),
        );
        ld_content = ld_content.replace("%ALIGN%", &format!("4K"));
    }

    ld_content = ld_content.replace("%SMP%", &format!("{}", axconfig::SMP));

    std::fs::write(fname, ld_content)?;
    Ok(())
}
