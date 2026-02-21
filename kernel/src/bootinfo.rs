#[repr(C)]
pub struct BootInfo {
    pub magic: u64,
    pub memory_map: *const MemoryMapEntry,
    pub memory_map_entries: usize,
    pub framebuffer: FramebufferInfo,
    pub arch: Architecture,
    pub kernel_start: u64,
    pub kernel_end: u64,
    pub initrd_start: u64,
    pub initrd_end: u64,
    pub cmdline: *const u8,
    pub cmdline_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FramebufferInfo {
    pub address: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u8,
    pub red_shift: u8,
    pub green_shift: u8,
    pub blue_shift: u8,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Architecture {
    X86 = 0,
    X86_64 = 1,
    Arm32 = 2,
    Arm64 = 3,
    Unknown = 255,
}

impl Architecture {
    /// Get current architecture at compile time
    pub fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Architecture::X86_64
        }
        #[cfg(target_arch = "x86")]
        {
            Architecture::X86
        }
        #[cfg(target_arch = "aarch64")]
        {
            Architecture::Arm64
        }
        #[cfg(target_arch = "arm")]
        {
            Architecture::Arm32
        }
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "arm"
        )))]
        {
            Architecture::Unknown
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryMapEntry {
    pub base: u64,
    pub length: u64,
    pub mem_type: MemoryType,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub enum MemoryType {
    Available = 1,
    Reserved = 2,
    AcpiReclaimable = 3,
    AcpiNvs = 4,
    BadMemory = 5,
    Kernel = 6,
    Bootloader = 7,
}
