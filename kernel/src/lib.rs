#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![allow(dead_code)]
#![allow(static_mut_refs)] // Kernel needs mutable statics for low-level hardware access
#![allow(unused_variables)] // Many syscall/driver stubs have unused parameters

mod arch;
mod bootinfo;

pub use bootinfo::BootInfo;

use crate::arch::serial::Serial;
use core::fmt::Write;

#[unsafe(no_mangle)]
pub extern "C" fn _start64(multiboot_info: u64) -> ! {
    let mut serial = Serial::default();
    serial.init();

    // Default! (text, not graphical)
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
                    serial.write_str("End of multiboot tags\n").unwrap();
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

                    serial
                        .write_fmt(format_args!(
                            "Framebuffer: addr={:#x}, {}x{}, pitch={}, bpp={}\n",
                            framebuffer_addr,
                            framebuffer_width,
                            framebuffer_height,
                            framebuffer_pitch,
                            framebuffer_bpp
                        ))
                        .unwrap();

                    // framebuffer types:
                    // - 0: indexed color (palette)
                    // - 1: RGB (this is what we want since we can write directly to it)
                    // - 2: EGA text

                    if fb_type != 1 {
                        serial
                            .write_fmt(format_args!("Unsupported framebuffer type: {}\n", fb_type))
                            .unwrap();

                        panic!("Unsupported framebuffer type");
                    }
                }

                addr += ((tag_size + 7) & !7) as u64; // align to 8 bytes
            }

            if framebuffer_addr == 0xb8000 {
                serial
                    .write_str("No framebuffer tag found, using defaults\n")
                    .unwrap();
            }
        }
    } else {
        serial
            .write_str("No multiboot info provided, using defaults\n")
            .unwrap();
    }

    // TODO: Move BootInfo parsing into a separate function and fill in the other fields (memory
    // map, cmdline, etc.)

    let boot_info = BootInfo {
        magic: multiboot_info,
        memory_map: core::ptr::null(),
        memory_map_entries: 0,
        framebuffer: bootinfo::FramebufferInfo {
            address: framebuffer_addr,
            width: framebuffer_width,
            height: framebuffer_height,
            pitch: framebuffer_pitch,
            bpp: framebuffer_bpp,
            red_shift: 16,
            green_shift: 8,
            blue_shift: 16,
        },
        arch: bootinfo::Architecture::current(),
        kernel_start: 0, // These would be filled in by the bootloader in a real implementation
        kernel_end: 0,
        initrd_start: 0,
        initrd_end: 0,
        cmdline: core::ptr::null(),
        cmdline_len: 0,
    };

    arch::halt();
    loop {}
}

// Reason for not test is because
// LSP screams about it!
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        arch::halt();
    }
}
