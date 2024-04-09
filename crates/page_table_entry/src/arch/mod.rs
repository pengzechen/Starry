

// TODO: `#[cfg(any(target_arch = "aarch64", doc))]` does not work.
#[doc(cfg(target_arch = "aarch64"))]
pub mod aarch64;

#[doc(cfg(target_arch = "aarch64", feature = "hv"))]
pub mod aarch64_hv;