use crate::proc::thread::Tid;
use alloc::vec::Vec;

pub type Pid = u64;

#[derive(Debug)]
pub struct Process {
    pub pid: Pid,
    pub cr3: u64,

    pub threads: Vec<Tid>,
}

impl Process {
    pub fn new(pid: Pid) -> Self {
        // TODO: required steps for making a process:
        // - allocate a page directory (cr3) (pml4, pdpt, pd, pt)
        // - set up the page tables to map the process's memory (code, data, stack)
        // - create a main thread for the process and add it to the threads vector

        log::trace!("Creating process with PID {}", pid);

        Self {
            pid,
            cr3: 0, // TODO: allocate a real page directory
            threads: Vec::new(),
        }
    }
}
