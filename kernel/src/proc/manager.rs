use crate::proc::process::{Pid, Process};

use alloc::vec::Vec;

const MAX_PROCESSES: usize = 1024;

// bitfield to track used pids
pub struct Manager {
    pub processes: Vec<Process>,
    process_bitmap: [u64; MAX_PROCESSES / 64],
}

impl Manager {
    pub const fn new() -> Self {
        let mut instance = Self {
            processes: Vec::new(),
            process_bitmap: [0; MAX_PROCESSES / 64],
        };

        // reserve PID 0 for the kernel process
        instance.process_bitmap[0] |= 1;

        instance
    }

    // TODO: don't take in cr3, allocate it auto
    pub fn create_process(&mut self) -> Pid {
        for (i, bitmap) in self.process_bitmap.iter_mut().enumerate() {
            if *bitmap != u64::MAX {
                for j in 0..64 {
                    let bit = 1 << j;

                    if (*bitmap & bit) == 0 {
                        *bitmap |= bit;
                        let pid = (i * 64 + j) as Pid;

                        self.processes.push(Process::new(pid));

                        log::trace!("Created process with PID {}", pid);
                        return pid;
                    }
                }
            }
        }

        panic!("No more PIDs available");
    }
}

static mut MANAGER: Manager = Manager::new();

pub fn get_manager() -> &'static mut Manager {
    unsafe { &mut MANAGER }
}

pub fn get_process(pid: Pid) -> Option<&'static Process> {
    get_manager().processes.iter().find(|p| p.pid == pid)
}
