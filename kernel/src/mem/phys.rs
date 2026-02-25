use crate::BootInfo;
use crate::mem::{MemoryType, PAGE_SIZE, page_align_down, page_align_up};
use spin::Mutex;

// TODO: Why not make this bigger? We can support more than 4 GiB of RAM, but we need to make sure
// our page tables can handle it
const MAX_PHYS_MEM: usize = 0x100000000; // 4 GiB

const MAX_PAGES: usize = MAX_PHYS_MEM / PAGE_SIZE;

const BITMAP_SIZE: usize = MAX_PAGES / 8; // 1 bit per page

/// The frame allocator allocates and deallocates physical memory frames (pages). It uses a bitmap
/// to track which frames are free or used.
///
/// The bitmap is an array of bytes, where each bit represents a page. A bit value of 0 indicates
/// that the corresponding page is free, while a bit value of 1 indicates that the page is
/// allocated.
///
/// A frame is a region of physical memory that is typically the size of a page (4 KiB).
pub struct FrameAllocator {
    bitmap: [u8; BITMAP_SIZE],
    first_free: usize,
    total_pages: usize,
    free_pages: usize,
}

impl FrameAllocator {
    pub const fn new() -> Self {
        Self {
            bitmap: [0; BITMAP_SIZE],
            first_free: 0,
            total_pages: 0,
            free_pages: 0,
        }
    }

    pub fn init(&mut self, boot_info: &BootInfo) {
        log::trace!("Initializing frame allocator");

        // Mark all pages as allocated
        for byte in self.bitmap.iter_mut() {
            *byte = 0xFF;
        }

        // If no memory map is provided, we have to assume all memory is available
        if boot_info.memory_map.is_null() || boot_info.memory_map_entries == 0 {
            log::warn!("No memory map provided, assuming all memory is available");

            self.total_pages = MAX_PAGES;
            self.free_pages = MAX_PAGES;

            return;
        } else {
            unsafe {
                for i in 0..boot_info.memory_map_entries {
                    let entry = &*boot_info.memory_map.add(i);

                    if entry.mem_type == MemoryType::Available {
                        let start = page_align_up(entry.base) as usize / PAGE_SIZE;
                        let end = page_align_down(entry.base + entry.length) as usize / PAGE_SIZE;

                        for page in start..end {
                            if page < MAX_PAGES {
                                self.mark_free(page);
                            }
                        }
                    }
                }
            }
        }

        log::debug!(
            "Frame allocator initialized: {} pages ({} MiB) total, {} pages ({} MiB) free",
            self.total_pages,
            (self.total_pages * PAGE_SIZE) / 1024 / 1024,
            self.free_pages,
            (self.free_pages * PAGE_SIZE) / 1024 / 1024,
        );
    }

    fn mark_free(&mut self, page: usize) {
        if page >= MAX_PAGES {
            return;
        }

        let byte = page / 8;
        let bit = page % 8;

        if !self.is_allocated(page) {
            return;
        }

        self.bitmap[byte] &= !(1 << bit);
        self.free_pages += 1;
        self.total_pages = self.total_pages.max(page + 1);
    }

    fn mark_allocated(&mut self, page: usize) {
        if page >= MAX_PAGES {
            return;
        }

        let byte = page / 8;
        let bit = page % 8;

        if self.is_allocated(page) {
            return;
        }

        self.bitmap[byte] |= 1 << bit;
        self.free_pages -= 1;
    }

    fn is_allocated(&self, page: usize) -> bool {
        if page >= MAX_PAGES {
            return true; // out of bounds pages are considered allocated
        }

        let byte = page / 8;
        let bit = page % 8;

        self.bitmap[byte] & (1 << bit) != 0
    }

    /// Allocate a single page and return its physical address. Returns None if no free pages are
    /// available.
    pub fn alloc(&mut self) -> Option<u64> {
        for page in self.first_free..self.total_pages {
            if !self.is_allocated(page) {
                self.mark_allocated(page);
                self.first_free = page + 1;
                return Some((page * PAGE_SIZE) as u64);
            }
        }

        // Wrap around and check from the beginning up to first_free
        for page in 0..self.first_free {
            if !self.is_allocated(page) {
                self.mark_allocated(page);
                self.first_free = page + 1;
                return Some((page * PAGE_SIZE) as u64);
            }
        }

        log::warn!(
            "Physical frame allocator out of memory: total={} pages, free={} pages",
            self.total_pages,
            self.free_pages
        );
        None // No free pages
    }

    pub fn alloc_contiguous(&mut self, num_pages: usize) -> Option<u64> {
        if num_pages == 0 || num_pages > self.free_pages {
            return None;
        }

        for start_page in self.first_free..=self.total_pages - num_pages {
            let mut found = true;

            for page in start_page..start_page + num_pages {
                if self.is_allocated(page) {
                    found = false;
                    break;
                }
            }

            if found {
                for page in start_page..start_page + num_pages {
                    self.mark_allocated(page);
                }
                self.first_free = start_page + num_pages;
                return Some((start_page * PAGE_SIZE) as u64);
            }
        }

        None // No contiguous block of free pages found
    }

    pub fn free(&mut self, addr: u64) {
        let page = (addr as usize) / PAGE_SIZE;

        if page < MAX_PAGES && self.is_allocated(page) {
            self.mark_free(page);
            if page < self.first_free {
                self.first_free = page; // Update first_free to the lowest free page
                // the reason we do this is that it prevents wraparounds in the alloc function.
            }
        }

        if page >= MAX_PAGES {
            log::warn!(
                "Attempted to free out-of-bounds page at address {:#x}",
                addr
            );
        }
    }

    pub fn free_contiguous(&mut self, addr: u64, num_pages: usize) {
        let start_page = (addr as usize) / PAGE_SIZE;

        for i in 0..num_pages {
            let page = start_page + i;
            if page < MAX_PAGES {
                self.mark_free(page);
            } else {
                log::warn!(
                    "Attempted to free out-of-bounds page at address {:#x}",
                    (page * PAGE_SIZE) as u64
                );
            }
        }

        if start_page < self.first_free {
            self.first_free = start_page; // Update first_free to the lowest free page
        }
    }

    pub fn free_count(&self) -> usize {
        self.free_pages
    }

    pub fn total_count(&self) -> usize {
        self.total_pages
    }
}

static FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new());

pub fn init(boot_info: &BootInfo) {
    FRAME_ALLOCATOR.lock().init(boot_info);
}

pub fn alloc_frame() -> Option<u64> {
    FRAME_ALLOCATOR.lock().alloc()
}

pub fn alloc_frames(count: usize) -> Option<u64> {
    FRAME_ALLOCATOR.lock().alloc_contiguous(count)
}

pub fn free_frame(addr: u64) {
    FRAME_ALLOCATOR.lock().free(addr);
}

pub fn free_frames(addr: u64, count: usize) {
    FRAME_ALLOCATOR.lock().free_contiguous(addr, count);
}

pub fn free_frames_count() -> usize {
    FRAME_ALLOCATOR.lock().free_count()
}

pub fn total_frames_count() -> usize {
    FRAME_ALLOCATOR.lock().total_count()
}

pub fn stats() -> (usize, usize, usize) {
    let allocator = FRAME_ALLOCATOR.lock();

    let total = allocator.total_count();
    let free = allocator.free_count();
    let used = total - free;

    (total, used, free)
}
