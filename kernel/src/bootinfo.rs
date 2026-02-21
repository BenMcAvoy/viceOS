#[repr(C)]
#[derive(Debug)]
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
#[derive(Clone, Copy, Debug)]
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

impl BootInfo {
    pub fn from_bootloader(multiboot_info: u64) -> Self {
        use core::fmt::Write;

        let mut framebuffer_addr: u64 = 0xb8000;
        let mut framebuffer_width: u32 = 80;
        let mut framebuffer_height: u32 = 25;
        let mut framebuffer_pitch: u32 = 160;
        let mut framebuffer_bpp: u8 = 16;

        if multiboot_info != 0 {
            unsafe {
                let total_size = *(multiboot_info as *const u32) as usize;
                let mut addr = multiboot_info + 8; // skip total_size & reserved
                let end = multiboot_info + total_size as u64;

                while addr < end {
                    let tag_type = *(addr as *const u32);
                    let tag_size = *((addr + 4) as *const u32) as usize;

                    if tag_type == 0 {
                        break; // End tag
                    }

                    // Framebuffer
                    if tag_type == 8 {
                        framebuffer_addr = *((addr + 8) as *const u64);
                        framebuffer_pitch = *((addr + 16) as *const u32);
                        framebuffer_width = *((addr + 20) as *const u32);
                        framebuffer_height = *((addr + 24) as *const u32);
                        framebuffer_bpp = *((addr + 28) as *const u8);

                        let fb_type = *((addr + 29) as *const u8);

                        // framebuffer types:
                        // - 0: indexed color (palette)
                        // - 1: RGB (this is what we want since we can write directly to it)
                        // - 2: EGA text

                        if fb_type != 1 {
                            panic!("Unsupported framebuffer type");
                        }
                    }

                    addr += ((tag_size + 7) & !7) as u64; // align to 8 bytes
                }
            }
        }

        BootInfo {
            magic: multiboot_info,
            memory_map: core::ptr::null(),
            memory_map_entries: 0,
            framebuffer: FramebufferInfo {
                address: framebuffer_addr,
                width: framebuffer_width,
                height: framebuffer_height,
                pitch: framebuffer_pitch,
                bpp: framebuffer_bpp,
                red_shift: 16,
                green_shift: 8,
                blue_shift: 16,
            },
            arch: Architecture::current(),
            kernel_start: 0,
            kernel_end: 0,
            initrd_start: 0,
            initrd_end: 0,
            cmdline: core::ptr::null(),
            cmdline_len: 0,
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
