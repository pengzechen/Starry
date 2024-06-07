


qemu-virt: 
make PLATFORM=qemu-virt-aarch64 LOG=debug SMP=1 GIC_V3=y HV=y GUEST=linux run
make PLATFORM=qemu-virt-aarch64 LOG=debug SMP=1 GIC_V3=n HV=y GUEST=linux run

make PLATFORM=qemu-virt-aarch64 LOG=debug SMP=1 GIC_V3=y HV=y GUEST=nimbos run
make PLATFORM=qemu-virt-aarch64 LOG=debug SMP=1 GIC_V3=n HV=y GUEST=nimbos run


rk3588:
make PLATFORM=rk3588-aarch64 LOG=debug SMP=1 GIC_V3=y HV=y GUEST=linux kernel
make PLATFORM=rk3588-aarch64 LOG=debug SMP=1 GIC_V3=n HV=y GUEST=linux kernel

make PLATFORM=rk3588-aarch64 LOG=debug SMP=1 GIC_V3=y HV=y GUEST=nimbos kernel
make PLATFORM=rk3588-aarch64 LOG=debug SMP=1 GIC_V3=n HV=y GUEST=nimbos kernel