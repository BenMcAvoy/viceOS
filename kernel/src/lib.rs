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

use crate::arch::serial;

#[unsafe(no_mangle)]
pub extern "C" fn _start64(multiboot_info: u64) -> ! {
    let serial = serial::Serial::default(); // COM1 port
    serial.init();

    serial.write_string("Hello, world! This is a test of the serial port.\n");

    loop {
        arch::halt();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        arch::halt();
    }
}
