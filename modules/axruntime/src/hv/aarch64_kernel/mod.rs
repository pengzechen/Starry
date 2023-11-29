pub mod vm_array;

pub use vm_array::{
    VM_ARRAY, VM_MAX_NUM, 
    is_vcpu_init_ok, is_vcpu_primary_ok, init_vm_vcpu, add_vm, print_vm, run_vm_vcpu
};