mod exception;
mod sync;
mod guest_psci;
mod ipi;

#[macro_export]
macro_rules! declare_enum_with_handler {
    (
        $enum_vis:vis enum $enum_name:ident [$array_vis:vis $array:ident => $handler_type:ty] {
            $($vis:vis $variant:ident => $handler:expr, )*
        }
    ) => {
        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        #[repr(usize)]
        $enum_vis enum $enum_name {
            $($vis $variant, )*
        }
        $array_vis static $array: &[$handler_type] = &[
            $($handler, )*
        ];
    }
}

#[macro_use]
pub use declare_enum_with_handler;