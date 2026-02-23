//use crate::mm::{PAGE_SIZE, physical};
use alloc::vec::Vec;
use spin::Mutex;

pub struct VmRegion {
    pub start: u64,
    pub end: u64,
    pub flags: VmFlags,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct VmFlags: u32 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXECUTE = 1 << 2;
        const USER = 1 << 3;
        const SHARED = 1 << 4;
        const STACK = 1 << 5;
        const HEAP = 1 << 6;
        const MMIO = 1 << 7;
    }
}
