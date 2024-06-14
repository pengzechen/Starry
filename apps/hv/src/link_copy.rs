const NIMBOS_DTB_SIZE: usize = 3303;
const NIMBOS_KERNEL_SIZE: usize = 710136;
const TSETOS_KERNEL_SIZE: usize = 372;

const NIMBOS_QEMU_DTB_SIZE: usize  = 3434;
const NIMBOS_QEMU_KERNEL_SIZE: usize = 552960;


pub const MEM_SIZE:usize =    0x8_0000;

#[link_section = ".guestdata.mem"]
static OS_MEM: [u8; MEM_SIZE] = [0; MEM_SIZE];

/* ======== nimbos ========= */

#[cfg(all(feature = "nimbos", feature = "platform-qemu-virt-aarch64"))]
#[link_section = ".guestdata.dtb"]
static OS_DTB: [u8; NIMBOS_QEMU_DTB_SIZE] = *include_bytes!("../guest/nimbos/nimbos-aarch64-v3.dtb");
#[cfg(all(feature = "nimbos", feature = "platform-qemu-virt-aarch64"))]
#[link_section = ".guestdata.kernel"]
static OS_KERNEL: [u8; NIMBOS_QEMU_KERNEL_SIZE] = *include_bytes!("../guest/nimbos/nimbos-aarch64-v3.bin");

#[cfg(all(feature = "nimbos", feature = "platform-rk3588-aarch64"))]
#[link_section = ".guestdata.dtb"]
static OS_DTB: [u8; NIMBOS_DTB_SIZE] = *include_bytes!("../guest/nimbos/nimbos-aarch64-v3-rk.dtb");
#[cfg(all(feature = "nimbos", feature = "platform-rk3588-aarch64"))]
#[link_section = ".guestdata.kernel"]
static OS_KERNEL: [u8; NIMBOS_KERNEL_SIZE] = *include_bytes!("../guest/nimbos/nimbos-aarch64-v3-asqemu.bin");


/* ========= linux ========= */

#[cfg(all(feature = "linux", feature = "platform-qemu-virt-aarch64"))]
#[link_section = ".guestdata.dtb"]
static OS_DTB: [u8; LINUX_QEMU_DTB_SIZE] = *include_bytes!("../guest/linux/linux-aarch64-v3.dtb");
#[cfg(all(feature = "linux", feature = "platform-qemu-virt-aarch64"))]
#[link_section = ".guestdata.kernel"]
static OS_KERNEL: [u8; LINUX_QEMU_KERNEL_SIZE] = *include_bytes!("../guest/linux/linux-aarch64-v3.bin");

#[cfg(all(feature = "linux", feature = "platform-rk3588-aarch64"))]
#[link_section = ".guestdata.dtb"]
static OS_DTB: [u8; LINUX_RK5588_DTB_SIZE] = *include_bytes!("../guest/linux/linux-aarch64-v3.dtb");
#[cfg(all(feature = "linux", feature = "platform-rk3588-aarch64"))]
#[link_section = ".guestdata.kernel"]
static OS_KERNEL: [u8; LINUX_RK5588_KERNEL_SIZE] = *include_bytes!("../guest/linux/linux-aarch64-v3.bin");


/* ========== testos ========== */

#[cfg(all(feature = "testos", feature = "platform-qemu-virt-aarch64"))]
#[link_section = ".guestdata.kernel"]
static OS_KERNEL: [u8; TSETOS_KERNEL_SIZE] = *include_bytes!("../guest/testos/testos.bin");
#[cfg(all(feature = "testos", feature = "platform-qemu-virt-aarch64"))]
#[link_section = ".guestdata.dtb"]
static OS_DTB: [u8; NIMBOS_QEMU_DTB_SIZE] = *include_bytes!("../guest/nimbos/nimbos-aarch64-v3.dtb");  // not use

#[cfg(all(feature = "testos", feature = "platform-rk3588-aarch64"))]
#[link_section = ".guestdata.kernel"]
static OS_KERNEL: [u8; TSETOS_KERNEL_SIZE] = *include_bytes!("../guest/testos/testos.bin");
#[cfg(all(feature = "testos", feature = "platform-rk3588-aarch64"))]
#[link_section = ".guestdata.dtb"]
static OS_DTB: [u8; NIMBOS_QEMU_DTB_SIZE] = *include_bytes!("../guest/nimbos/nimbos-aarch64-v3.dtb");  // not use


pub const DTB_COPYED_ADDR:usize =    0x7000_0000;
pub const KERNEL_COPYED_ADDR:usize = 0x7020_0000;


extern "C" {
    fn __guest_dtb_start();
    fn __guest_dtb_end();
    fn __guest_kernel_start();
    fn __guest_kernel_end();
}

// 保证这俩函数运行，不然NIMBOS_DTB NIMBOS_KERNEL会被优化掉
fn test_dtbdata() {
    // 地址转换为指针
    let address: *const u8 = OS_DTB.as_ptr() as usize as * const u8;

    // 创建一个长度为10的数组来存储读取的数据
    let mut buffer = [0u8; 20];

    unsafe {
        // 从指定地址读取10个字节
        for i in 0..20 {
            buffer[i] = *address.offset(i as isize);
        }
    }

    // 输出读取的数据
    debug!("{:?}", buffer);
}

fn test_kerneldata() {
    // 地址转换为指针
    let address: *const u8 = OS_KERNEL.as_ptr() as usize as * const u8;

    // 创建一个长度为10的数组来存储读取的数据
    let mut buffer = [0u8; 20];

    unsafe {
        // 从指定地址读取10个字节
        for i in 0..20 {
            buffer[i] = *address.offset(i as isize);
        }
    }

    // 输出读取的数据
    debug!("{:?}", buffer);
}

fn test_dtbdata_high() {
    // 地址转换为指针
    let address: *const u8 = DTB_COPYED_ADDR as * const u8;

    // 创建一个长度为10的数组来存储读取的数据
    let mut buffer = [0u8; 20];

    unsafe {
        // 从指定地址读取10个字节
        for i in 0..20 {
            buffer[i] = *address.offset(i as isize);
        }
    }

    // 输出读取的数据
    debug!("{:?}", buffer);
}

fn test_kerneldata_high() {
    // 地址转换为指针
    let address: *const u8 = KERNEL_COPYED_ADDR as * const u8;

    // 创建一个长度为10的数组来存储读取的数据
    let mut buffer = [0u8; 20];

    unsafe {
        // 从指定地址读取10个字节
        for i in 0..20 {
            buffer[i] = *address.offset(i as isize);
        }
    }

    // 输出读取的数据
    debug!("{:?}", buffer);
}

pub fn copy_high_data() -> usize {

    //  申请一块内存  大小为 memory 大小
    // use alloc::alloc::Layout;
    // let layout = Layout::from_size_align(NIMBOS_MEM_SIZE, 8192).unwrap();
    // let area_base: *mut u8 = unsafe { alloc::alloc::alloc_zeroed(layout) };
    // info!("base: {:#x}, layout size: {:#x}", area_base as usize, layout.size());

    //zero data

    let area_base  = DTB_COPYED_ADDR as * mut u8;

    unsafe {
        // 从指定地址读取10个字节
        for i in 0..MEM_SIZE {
            *area_base.offset(i as isize) = 0;
        }
    }


    let area_base  = DTB_COPYED_ADDR as * mut u8;

    let tls_load_base = __guest_dtb_start as *mut u8;
    let tls_load_size = __guest_dtb_end as usize - __guest_dtb_start as usize;
    unsafe {
        // copy data from .tbdata section
        core::ptr::copy_nonoverlapping(
            tls_load_base,
            area_base,
            tls_load_size,
        );
    }

    let area_base  = KERNEL_COPYED_ADDR as * mut u8;

    let tls_load_base = __guest_kernel_start as *mut u8;
    let tls_load_size = __guest_kernel_end as usize - __guest_kernel_start as usize;
    unsafe {
        // copy data from .tbdata section
        core::ptr::copy_nonoverlapping(
            tls_load_base,
            area_base,
            tls_load_size,
        );
    }

    debug!("{}", OS_MEM[0]);
    test_dtbdata();
    test_kerneldata();

    test_dtbdata_high();
    test_kerneldata_high();
    area_base as usize
}