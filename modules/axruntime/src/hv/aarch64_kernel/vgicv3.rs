



use crate::{GuestPageTable, HyperCraftHalImpl};

fn remove_int_list(&self, vcpu: Vcpu, interrupt: VgicInt, is_pend: bool) {
    let mut cpu_priv = self.cpu_priv.lock();
    let vcpu_id = vcpu.id();
    let int_id = interrupt.id();
    if interrupt.in_lr() {
        if is_pend {
            if !interrupt.in_pend() {
                return;
            }
            for i in 0..cpu_priv[vcpu_id].pend_list.len() {
                if cpu_priv[vcpu_id].pend_list[i].id() == int_id {
                    cpu_priv[vcpu_id].pend_list.remove(i);
                    break;
                }
            }
            interrupt.set_in_pend_state(false);
        } else {
            if !interrupt.in_act() {
                return;
            }
            for i in 0..cpu_priv[vcpu_id].act_list.len() {
                if cpu_priv[vcpu_id].act_list[i].id() == int_id {
                    cpu_priv[vcpu_id].act_list.remove(i);
                    break;
                }
            }
            interrupt.set_in_act_state(false);
        };
    }
}