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

#[unsafe(no_mangle)]
pub extern "C" fn _start64(multiboot_info: u64) -> ! {
    let boot_info = BootInfo::from_bootloader(multiboot_info);
    kprintln!("BootInfo: {:#?}", boot_info);

    arch::init(&boot_info);
    kernel_main(&boot_info);
}

pub extern "C" fn kernel_main(boot_info: *const BootInfo) -> ! {
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

#[macro_export]
macro_rules! kprintln {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}
