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

    let boot_info = BootInfo::from_bootloader(multiboot_info);

    writeln!(serial, "BootInfo: {:#?}", boot_info).unwrap();

    loop {
        arch::halt();
    }
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
