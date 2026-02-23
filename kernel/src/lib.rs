#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![allow(dead_code)]
#![allow(static_mut_refs)] // Kernel needs mutable statics for low-level hardware access
#![allow(unused_variables)] // Many syscall/driver stubs have unused parameters

mod arch;
mod bootinfo;
mod logging;

pub use bootinfo::BootInfo;

use log::LevelFilter;

#[unsafe(no_mangle)]
pub extern "C" fn _start64(multiboot_info: u64) -> ! {
    logging::init(LevelFilter::Trace).expect("Failed to initialize logger");

    let boot_info = BootInfo::from_bootloader(multiboot_info);
    log::debug!("BootInfo: {:?}", boot_info);

    arch::init(&boot_info);
    kernel_main(&boot_info);
}

pub extern "C" fn kernel_main(boot_info: *const BootInfo) -> ! {
    log::info!("Entering kernel main");

    // Draw checkerboard pattern to framebuffer
    unsafe {
        let fb_addr = (*boot_info).framebuffer.address as *mut u32;
        let fb_width = (*boot_info).framebuffer.width as usize;
        let fb_height = (*boot_info).framebuffer.height as usize;

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

    loop {
        arch::halt();
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
