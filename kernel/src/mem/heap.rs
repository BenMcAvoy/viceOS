use crate::mem::{PAGE_SIZE, phys};
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use linked_list_allocator::LockedHeap;
use spin::Mutex;

const HEAP_START: u64 = 0x0000_0000_0200_0000; // 32 MiB, past the kernel and bootloader
const INITIAL_HEAP_SIZE: usize = 4 * 1024 * 1024; // 4 MiB initial heap
const EXTEND_CHUNK_SIZE: usize = 4 * 1024 * 1024; // grow by 4 MiB at a time (minimum)
const MAX_HEAP_SIZE: usize = 512 * 1024 * 1024; // 512 MiB hard cap

/// Heap allocator that automatically extends itself when an allocation fails.
struct AutoExtendHeap {
    inner: LockedHeap,
    /// Tracks the current end of the mapped heap region.
    heap_end: Mutex<u64>,
}

impl AutoExtendHeap {
    const fn new() -> Self {
        Self {
            inner: LockedHeap::empty(),
            heap_end: Mutex::new(HEAP_START),
        }
    }

    fn init(&self) {
        let mut heap_end = self.heap_end.lock();
        let num_pages = (INITIAL_HEAP_SIZE + PAGE_SIZE - 1) / PAGE_SIZE;

        for i in 0..num_pages {
            let phys = phys::alloc_frame().expect("Failed to allocate frame for initial heap");
            let virt = HEAP_START + (i * PAGE_SIZE) as u64;
            use crate::arch::paging::{self, flags};
            paging::map_page(virt, phys, flags::PRESENT | flags::WRITABLE)
                .expect("Failed to map heap page");
        }

        let mapped = (num_pages * PAGE_SIZE) as u64;
        *heap_end = HEAP_START + mapped;

        unsafe {
            self.inner
                .lock()
                .init(HEAP_START as *mut u8, num_pages * PAGE_SIZE);
        }

        log::trace!(
            "Heap initialized at {:#x}, size {} KiB",
            HEAP_START,
            (num_pages * PAGE_SIZE) / 1024
        );
    }

    /// Map more pages into the heap and tell the inner allocator about them.
    /// Extends by at least `min_bytes` (rounded up to pages), but at least
    /// `EXTEND_CHUNK_SIZE` so we don't thrash on many small extensions.
    fn try_extend(&self, min_bytes: usize) -> bool {
        let mut heap_end = self.heap_end.lock();
        let current_size = (*heap_end - HEAP_START) as usize;

        if current_size >= MAX_HEAP_SIZE {
            log::warn!(
                "Heap has reached maximum size ({} MiB)",
                MAX_HEAP_SIZE / 1024 / 1024
            );
            return false;
        }

        let want = min_bytes.max(EXTEND_CHUNK_SIZE);
        let capped = want.min(MAX_HEAP_SIZE - current_size);
        let num_pages = (capped + PAGE_SIZE - 1) / PAGE_SIZE;

        let mut mapped_pages = 0usize;
        for i in 0..num_pages {
            let frame = match phys::alloc_frame() {
                Some(f) => f,
                None => {
                    log::warn!(
                        "Heap extension stopped early: out of physical frames after {} pages",
                        i
                    );
                    break;
                }
            };

            let virt = *heap_end + (i * PAGE_SIZE) as u64;
            use crate::arch::paging::{self, flags};
            match paging::map_page(virt, frame, flags::PRESENT | flags::WRITABLE) {
                Ok(_) => mapped_pages += 1,
                Err(_) => {
                    phys::free_frame(frame);
                    log::warn!(
                        "Heap extension stopped early: failed to map virt {:#x}",
                        virt
                    );
                    break;
                }
            }
        }

        if mapped_pages == 0 {
            return false;
        }

        let added = mapped_pages * PAGE_SIZE;
        unsafe {
            self.inner.lock().extend(added);
        }
        *heap_end += added as u64;

        log::debug!(
            "Heap extended by {} KiB (total: {} KiB / {} MiB max)",
            added / 1024,
            (*heap_end - HEAP_START) as usize / 1024,
            MAX_HEAP_SIZE / 1024 / 1024,
        );

        true
    }
}

unsafe impl GlobalAlloc for AutoExtendHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self
            .inner
            .lock()
            .allocate_first_fit(layout)
            .ok()
            .map_or(core::ptr::null_mut(), NonNull::as_ptr);

        if !ptr.is_null() {
            return ptr;
        }

        // First attempt failed - try to grow the heap and retry once.
        if self.try_extend(layout.size()) {
            self.inner
                .lock()
                .allocate_first_fit(layout)
                .ok()
                .map_or(core::ptr::null_mut(), NonNull::as_ptr)
        } else {
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            self.inner
                .lock()
                .deallocate(NonNull::new_unchecked(ptr), layout);
        }
    }
}

#[global_allocator]
static ALLOCATOR: AutoExtendHeap = AutoExtendHeap::new();

pub fn init() {
    ALLOCATOR.init();
}

/// Get heap statistics: (free, used)
pub fn heap_stats() -> (usize, usize) {
    let inner = ALLOCATOR.inner.lock();
    (inner.free(), inner.used())
}

/// Get current mapped heap size in bytes
pub fn heap_size() -> usize {
    (*ALLOCATOR.heap_end.lock() - HEAP_START) as usize
}
