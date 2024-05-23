# QEMU arguments

QEMU := qemu-system-$(ARCH)

GUEST ?= linux

ROOTFS ?= apps/hv/guest/$(GUEST)/rootfs.img

ifeq ($(ARCH), aarch64)
  ROOTFS = apps/hv/guest/$(GUEST)/rootfs-aarch64.img
  GUEST_DTB = apps/hv/guest/$(GUEST)/$(GUEST)-aarch64-v3.dtb
  GUEST_BIN = apps/hv/guest/$(GUEST)/$(GUEST)-aarch64-v3.bin
endif


ifeq ($(BUS), mmio)
  vdev-suffix := device
else ifeq ($(BUS), pci)
  vdev-suffix := pci
else
  $(error "BUS" must be one of "mmio" or "pci")
endif

qemu_args-aarch64 := \
  -cpu cortex-a72 \
  -machine virt \
  -kernel $(OUT_BIN)

ifeq ($(HV), y)
  ifeq ($(ARCH), aarch64)
    qemu_args-y := \
        -m 3G -smp $(SMP) $(qemu_args-$(ARCH)) \
    	  -device loader,file=$(GUEST_DTB),addr=0x70000000,force-raw=on \
        -device loader,file=$(GUEST_BIN),addr=0x70200000,force-raw=on \
        -machine virtualization=on,gic-version=3,secure=on
  endif
else
  qemu_args-y := -m 128M -smp $(SMP) $(qemu_args-$(ARCH))
endif

qemu_args-$(FS) += \
  -device virtio-blk-$(vdev-suffix),drive=disk0 \
  -drive id=disk0,if=none,format=raw,file=$(DISK_IMG)

qemu_args-$(NET) += \
  -device virtio-net-$(vdev-suffix),netdev=net0 \
  -netdev user,id=net0,hostfwd=tcp::5555-:5555,hostfwd=udp::5555-:5555

ifeq ($(NET_DUMP), y)
  qemu_args-$(NET) += -object filter-dump,id=dump0,netdev=net0,file=qemu-net0.pcap
endif

qemu_args-$(GRAPHIC) += \
  -device virtio-gpu-$(vdev-suffix) -vga none \
  -serial mon:stdio

ifeq ($(GUEST), linux)
  ifeq ($(ARCH), aarch64)
    qemu_args-$(HV) += \
      -drive if=none,file=$(ROOTFS),format=raw,id=hd0 \
	    -device virtio-blk-device,drive=hd0 \
	    # -append "root=/dev/vda rw console=ttyAMA0"
  endif
else ifeq ($(GUEST), rCore-Tutorial)
  qemu_args-$(HV) += \
    	-drive file=guest/rCore-Tutorial-v3/fs.img,if=none,format=raw,id=x0 \
	    -device virtio-blk-device,drive=x0 \
      -device virtio-gpu-device \
      -device virtio-keyboard-device \
      -device virtio-mouse-device \
      -device virtio-net-device,netdev=net0 \
      -netdev user,id=net0,hostfwd=udp::6200-:2000
endif


ifeq ($(GRAPHIC), n)
  qemu_args-y += -nographic
endif

ifeq ($(QEMU_LOG), y)
  qemu_args-y += -D qemu.log -d in_asm,int,mmu,pcall,cpu_reset,guest_errors
endif

qemu_args-debug := $(qemu_args-y) -s -S

# Do not use KVM for debugging
ifeq ($(shell uname), Darwin)
  qemu_args-$(ACCEL) += -cpu host -accel hvf
else
  qemu_args-$(ACCEL) += -cpu host -accel kvm
endif

define run_qemu
  @printf "    $(CYAN_C)Running$(END_C) $(QEMU) $(qemu_args-y) $(1)\n"
  @$(QEMU) $(qemu_args-y)
endef

define run_qemu_debug
  @printf "    $(CYAN_C)Running$(END_C) $(QEMU) $(qemu_args-debug) $(1)\n"
  @$(QEMU) $(qemu_args-debug)
endef
