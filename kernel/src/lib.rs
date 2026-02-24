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

pub use bootinfo::{BootInfo, FramebufferInfo};

use log::LevelFilter;

use crate::drivers::screen::SCREEN;

use libm::{cos, sin};

const KERNEL_BANNER: &str = r#"
         oo                    .88888.  .d88888b  
                              d8'   `8b 88.    "' 
dP   .dP dP .d8888b. .d8888b. 88     88 `Y88888b. 
88   d8' 88 88'  `"" 88ooood8 88     88       `8b 
88 .88'  88 88.  ... 88.  ... Y8.   .8P d8'   .8P 
8888P'   dP `88888P' `88888P'  `8888P'   Y88888P  

   Welcome to viceOS, a hobby OS written in Rust!
"#;

#[unsafe(no_mangle)]
pub extern "C" fn _start64(multiboot_info: u64) -> ! {
    logging::init(LevelFilter::Trace).expect("Failed to initialize logger");

    let boot_info = BootInfo::from_bootloader(multiboot_info);
    arch::init(&boot_info);

    log::trace!("Entering kernel main");
    kernel_main(&boot_info);
}

pub extern "C" fn kernel_main(boot_info: &BootInfo) -> ! {
    mem::init(boot_info);
    drivers::init(boot_info);

    kprintln!("{}", KERNEL_BANNER);

    let mut screen = SCREEN.lock();

    let screen_width = screen.width;
    let screen_height = screen.height;

    let midx = screen.width as f64 / 2.0;
    let midy = screen.height as f64 / 2.0;

    let mut counter: u64 = 0;

    loop {
        use tiny_skia::*;

        let mut pixmap = PixmapMut::from_bytes(
            screen.get_buffer(),
            screen_width as u32,
            screen_height as u32,
        )
        .unwrap();

        pixmap.fill(Color::WHITE);

        let mut pb = PathBuilder::new();

        let x = midx + 100.0 * cos((counter as f32 * 0.01).into());
        let y = midy + 100.0 * sin((counter as f32 * 0.01).into());

        pb.push_circle(x as f32, y as f32, 100.0);

        counter = counter.wrapping_add(1);

        let path = pb.finish().unwrap();

        let mut paint = Paint::default();
        paint.set_color_rgba8(0, 255, 0, 255);

        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );

        screen.sync();
    }

    /*loop {
        arch::halt();
    }*/
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
