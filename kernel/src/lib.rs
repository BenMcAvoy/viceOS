#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![allow(dead_code)]
#![allow(static_mut_refs)] // Kernel needs mutable statics for low-level hardware access
#![allow(unused_variables)] // Many syscall/driver stubs have unused parameters

extern crate alloc;

mod arch;
mod bootinfo;
mod logging;
mod mem;

pub use bootinfo::{BootInfo, FramebufferInfo};

use log::LevelFilter;

use alloc::vec::Vec;

#[unsafe(no_mangle)]
pub extern "C" fn _start64(multiboot_info: u64) -> ! {
    logging::init(LevelFilter::Trace).expect("Failed to initialize logger");

    let boot_info = BootInfo::from_bootloader(multiboot_info);
    log::debug!("BootInfo: {:?}", boot_info);

    arch::init(&boot_info);
    mem::init(&boot_info);

    kernel_main(&boot_info);
}

pub extern "C" fn kernel_main(boot_info: *const BootInfo) -> ! {
    log::info!("Entering kernel main");

    draw_checkerboard(unsafe { &(*boot_info).framebuffer });

    // lets try to allocate a vector of 16384 KiB to test the allocator
    let mut vec: Vec<u32> = Vec::with_capacity(8192 * 520); // 4096 * 4 bytes = 16384 KiB
    for i in 0..8192 * 520 {
        vec.push(i as u32);
    }
    log::info!("Allocated vector with {} elements", vec.len());

    let heap_status = mem::phys::stats();
    let (free_frames, total_frames, used_frames) = heap_status;
    log::info!(
        "Physical memory: {} frames total, {} frames free, {} frames used",
        total_frames,
        free_frames,
        used_frames
    );

    let heap_status = mem::heap::heap_stats();
    let (free_heap, used_heap) = heap_status;
    log::info!(
        "Heap memory: {} bytes free, {} bytes used",
        free_heap,
        used_heap
    );

    loop {
        arch::halt();
    }
}

fn draw_checkerboard(fb: &FramebufferInfo) {
    unsafe {
        let fb_addr = fb.address as *mut u32;
        let fb_width = fb.width as usize;
        let fb_height = fb.height as usize;

        for y in 0..fb_height {
            for x in 0..fb_width {
                let color = if (x / 50 + y / 50) % 2 == 0 {
                    0xFF0000
                } else {
                    0x00FF00
                };
                *fb_addr.add(y * fb_width + x) = color;
            }
        }
    }
}

// Reason for not test is because
// LSP screams about it!
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    log::error!("Kernel panic: {}", _info);

    loop {
        arch::halt();
    }
}

#[macro_export]
macro_rules! kprintln {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    let (heap_free, heap_used) = mem::heap::heap_stats();
    let heap_total = mem::heap::heap_size();
    let (phys_total, phys_used, phys_free) = mem::phys::stats();

    log::error!("Allocation failed: size={}, align={}", layout.size(), layout.align());
    log::error!(
        "Heap:  total={} KiB, used={} KiB, free={} KiB",
        heap_total / 1024,
        heap_used / 1024,
        heap_free / 1024
    );
    log::error!(
        "Phys:  total={} pages, used={} pages, free={} pages",
        phys_total,
        phys_used,
        phys_free
    );

    panic!("Allocation error: {:?}", layout);
}
