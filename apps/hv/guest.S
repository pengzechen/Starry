
    .section .data
    .global guestkernel_start
    .global guestkernel_end
    .align 16
guestkernel_start:
    .incbin "./apps/hv/guest/nimbos/nimbos-aarch64_rk3588.bin"
guestkernel_end:

  .section .data
    .global guestdtb_start
    .global guestdtb_end
    .align 16
guestdtb_start:
    .incbin "./apps/hv/guest/nimbos/nimbos-aarch64_rk3588.dtb"
guestdtb_end:
