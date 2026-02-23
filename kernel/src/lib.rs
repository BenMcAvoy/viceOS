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
mod drivers;
mod logging;
mod mem;
mod test_render;

pub use bootinfo::{BootInfo, FramebufferInfo};

use log::LevelFilter;

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

    let framebuffer = unsafe { (*boot_info).framebuffer };
    test_render::test_render_loop(framebuffer);
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

    log::error!(
        "Allocation failed: size={}, align={}",
        layout.size(),
        layout.align()
    );
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
