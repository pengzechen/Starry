
1.nimbos-aarch64-v3-rk.bin >> 
在rk3588使用
dwuart  
gicv3

phys-memory-base = "0x4000_0000"
phys-memory-size = "0x400_0000"     # 64M
kernel-base-paddr = "0x4008_0000"
kernel-base-vaddr = "0xffff_0000_4008_0000"
mmio-regions = [
    ["0xfeb50000", "0x1000"],      # 8250 UART
    ["0xfe600000", "0x10000"],   #v3: gicd
    ["0xfe680000", "0x20000"],   #v3: gicr
]

2.nimbos-aarch64-v3.bin >> 
在qemu下使用
pl011uart
gicv3

phys-memory-base = "0x4000_0000"
phys-memory-size = "0x400_0000"     # 64M
kernel-base-paddr = "0x4008_0000"
kernel-base-vaddr = "0xffff_0000_4008_0000"
mmio-regions = [
    ["0x0900_0000", "0x1000"],      # PL011 UART
    # ["0x0800_0000", "0x2_0000"],    # GICv2
    ["0x0800_0000", "0x2_0000"],   #v3: gicd
    ["0x080a_0000", "0x2_0000"],   #v3: gicr
]