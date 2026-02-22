//! The IDT is the Interrupt Descriptor Table
//! This allows the CPU to know where to jump when an interrupt or exception occurs. It contains
//! entries that correspond to vectors 0-255, which can be used for hardware interrupts, software
//! interrupts, and exceptions.

use crate::arch::x86_64::gdt::KERNEL_CODE_SELECTOR;
use core::mem::size_of;

/// IDT entry type
/// An interrupt clears the IF flag, while a trap does not. This means that interrupts can be
/// interrupted by other interrupts, while traps cannot.
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum GateType {
    Interrupt = 0xE,
    Trap = 0xF,
}

/// IDT entry structure
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn null() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    fn new(handler: u64, selector: u16, ist: u8, gate_type: GateType, dpl: u8) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector,
            ist,
            type_attr: (1 << 7) | ((dpl & 0b11) << 5) | (gate_type as u8),
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }

    fn set_handler(&mut self, handler: u64) {
        self.offset_low = (handler & 0xFFFF) as u16;
        self.offset_mid = ((handler >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler >> 32) & 0xFFFFFFFF) as u32;
        self.selector = KERNEL_CODE_SELECTOR;
        self.type_attr = (1 << 7) | GateType::Interrupt as u8;
    }
}

/// IDT descriptor
#[repr(C, packed)]
struct IdtDescriptor {
    size: u16,
    offset: u64,
}

/// IDT structure (256 entries)
#[repr(C, align(16))]
struct Idt {
    entries: [IdtEntry; 256],
}

/// Global IDT instance
static mut IDT: Idt = Idt {
    entries: [IdtEntry::null(); 256],
};

/// Initialize IDT
pub fn init() {}
