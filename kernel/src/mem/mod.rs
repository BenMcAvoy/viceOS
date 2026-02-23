pub mod heap;
pub mod phys;
pub mod virt;

use crate::BootInfo;
use spin::Mutex;

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SHIFT: usize = 12;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryType {
    Available = 1,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    BadMemory,
    Kernel,
    Bootloader,
    Framebuffer,
    PageTable,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryMapEntry {
    pub base: u64,
    pub length: u64,
    pub mem_type: MemoryType,
}

/// Memory statistics structure
/// This is given to us by multiboot
/// it lets us track how much memory we have, how much is used, and how many pages are free/used
/// this is essential for the kernel to manage memory effectively and to provide information to
/// user-space applications about available resources.
pub struct MemoryStats {
    pub total_memory: u64,
    pub available_memory: u64,
    pub used_memory: u64,
    pub free_pages: u64,
    pub used_pages: u64,
}

/// Global memory statistics (global instance)
static MEMORY_STATS: Mutex<MemoryStats> = Mutex::new(MemoryStats {
    total_memory: 0,
    available_memory: 0,
    used_memory: 0,
    free_pages: 0,
    used_pages: 0,
});

pub fn init(boot_info: &BootInfo) {
    log::trace!("Initializing memory management");
    parse_mem_map(boot_info);
    phys::init(boot_info);
    heap::init();
    log::info!(
        "Heap initialized: {} KiB",
        heap::heap_size() / 1024
    );
}

fn parse_mem_map(boot_info: &BootInfo) {
    let mut stats = MEMORY_STATS.lock();

    if boot_info.memory_map.is_null() || boot_info.memory_map_entries == 0 {
        // 32MB is a relatively safe assumption for the minimum amount of memory available on
        // modern systems, and it allows the kernel to function even without a memory map provided
        // by the bootloader. If the system has less than 32MB of memory, the kernel may encounter
        // issues, but this is a reasonable fallback for systems that do not provide a memory map.

        log::error!("No memory map provided by bootloader, assuming 32MB available");

        stats.total_memory = 32 * 1024 * 1024; // 32MB
        stats.available_memory = stats.total_memory;

        return;
    }

    unsafe {
        for i in 0..boot_info.memory_map_entries {
            let entry = &*boot_info.memory_map.add(i);

            // Only count actual RAM-backed regions toward total; reserved/MMIO
            // entries cover huge holes in the physical address space and would
            // make the total misleadingly large.
            let is_ram = matches!(
                entry.mem_type,
                MemoryType::Available
                    | MemoryType::AcpiReclaimable
                    | MemoryType::AcpiNvs
                    | MemoryType::Kernel
                    | MemoryType::Bootloader
            );

            if is_ram {
                stats.total_memory += entry.length;
            }

            if entry.mem_type == MemoryType::Available {
                stats.available_memory += entry.length;
            }
        }
    }

    log::debug!(
        "Memory map parsed: total = {} MB, available = {} MB",
        stats.total_memory / (1024 * 1024),
        stats.available_memory / (1024 * 1024)
    );
}

// Helpers

/// Align address down to page boundary
#[inline]
pub const fn page_align_down(addr: u64) -> u64 {
    addr & !(PAGE_SIZE as u64 - 1)
}

/// Align address up to page boundary
#[inline]
pub const fn page_align_up(addr: u64) -> u64 {
    (addr + PAGE_SIZE as u64 - 1) & !(PAGE_SIZE as u64 - 1)
}

/// Convert address to page number
#[inline]
pub const fn addr_to_page(addr: u64) -> u64 {
    addr >> PAGE_SHIFT
}

/// Convert page number to address
#[inline]
pub const fn page_to_addr(page: u64) -> u64 {
    page << PAGE_SHIFT
}
