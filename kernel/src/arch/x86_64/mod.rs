pub mod apic;
pub mod gdt;
pub mod idt;

pub mod serial;

use crate::BootInfo;
use log;

pub fn init(_: &BootInfo) {
    // TODO: pit init
    gdt::init();
    idt::init();
    serial::init();

    crate::arch::enable_interrupts();

    log::info!("Architecture initialized");
}

/// Read MSR (Model Specific Register)
/// From here we can get data such as TSC (Time Stamp Counter), APIC base, etc.
#[inline]
pub fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Write MSR (Model Specific Register)
#[inline]
pub fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nomem, nostack)
        );
    }
}

/// Read CR0 register
/// This register controls various aspects of the CPU's operation, such as enabling protected mode,
/// paging, etc.
#[inline]
pub fn read_cr0() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr0", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write CR0 register
#[inline]
pub fn write_cr0(value: u64) {
    unsafe {
        core::arch::asm!("mov cr0, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CR2 register (page fault address)
/// This register contains the linear address that caused a page fault, which can be useful for
/// handling page faults.
#[inline]
pub fn read_cr2() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read CR3 register (page table base)
#[inline]
pub fn read_cr3() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write CR3 register
#[inline]
pub fn write_cr3(value: u64) {
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CR4 register
/// This register controls various features of the CPU, such as enabling PAE (Physical Address
/// Extension),
#[inline]
pub fn read_cr4() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr4", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write CR4 register
#[inline]
pub fn write_cr4(value: u64) {
    unsafe {
        core::arch::asm!("mov cr4, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Invalidate TLB entry for address
/// This is used to ensure that changes to page tables are reflected in the TLB (Translation
/// Lookaside Buffer),
#[inline]
pub fn invlpg(addr: u64) {
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) addr, options(nostack));
    }
}

/// Get CPU features using CPUID
pub fn cpuid(leaf: u32) -> (u32, u32, u32, u32) {
    let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
    unsafe {
        core::arch::asm!(
            "push rbx",
            "cpuid",
            "mov {ebx_out:e}, ebx",
            "pop rbx",
            inout("eax") leaf => eax,
            ebx_out = out(reg) ebx,
            out("ecx") ecx,
            out("edx") edx,
            options(nomem, nostack)
        );
    }
    (eax, ebx, ecx, edx)
}

/// Read from port
#[inline]
pub fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!(
            "in al, dx",
            out("al") value,
            in("dx") port,
            options(nomem, nostack)
        );
    }
    value
}

/// Write to port
#[inline]
pub fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack)
        );
    }
}

/// Read 16-bit value from port
#[inline]
pub fn inw(port: u16) -> u16 {
    let value: u16;
    unsafe {
        core::arch::asm!(
            "in ax, dx",
            out("ax") value,
            in("dx") port,
            options(nomem, nostack)
        );
    }
    value
}

/// Write 16-bit value to port
#[inline]
pub fn outw(port: u16, value: u16) {
    unsafe {
        core::arch::asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") value,
            options(nomem, nostack)
        );
    }
}

/// Read 32-bit value from port
#[inline]
pub fn inl(port: u16) -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!(
            "in eax, dx",
            out("eax") value,
            in("dx") port,
            options(nomem, nostack)
        );
    }
    value
}

/// Write 32-bit value to port
#[inline]
pub fn outl(port: u16, value: u32) {
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") value,
            options(nomem, nostack)
        );
    }
}
