use std::{
    fs::File,
    io::{Result, Write},
};

fn new_guest_img() -> Result<()> {
    let mut f = File::create("./guest.S").unwrap();
    let img_path: &str = "./apps/hv/guest/nimbos/nimbos-aarch64_rk3588.bin";
    let dtb_path: &str = "./apps/hv/guest/nimbos/nimbos-aarch64_rk3588.dtb";

    writeln!(
        f,
        r#"
    .section .data
    .global guestkernel_start
    .global guestkernel_end
    .align 16
guestkernel_start:
    .incbin "{}"
guestkernel_end:

  .section .data
    .global guestdtb_start
    .global guestdtb_end
    .align 16
guestdtb_start:
    .incbin "{}"
guestdtb_end:"#,
    img_path,
    dtb_path
    )?;
    Ok(())
}

fn main() {
    if axconfig::FAMILY == "aarch64-rk3588j" {
         new_guest_img().unwrap();
    }
    println!("cargo:rustc-cfg=platform_family=\"{}\"", axconfig::FAMILY);
}
