use crate::proc::context::Context;
use crate::proc::process::Pid;

pub type Tid = u64;

pub struct Thread {
    pub tid: Tid,

    pub context: Context,
    pub parent_pid: Pid,

    // heap allocated kernel stack for syscalls
    pub kernel_stack: *mut u8,
}
