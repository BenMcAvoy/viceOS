//! The IDT is the Interrupt Descriptor Table
//! This allows the CPU to know where to jump when an interrupt or exception occurs. It contains
//! entries that correspond to vectors 0-255, which can be used for hardware interrupts, software
//! interrupts, and exceptions.

use crate::arch::x86_64::gdt::KERNEL_CODE_SELECTOR;
use crate::kprintln;

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

// ISR definitions

/// Saved register state and CPU-pushed frame for exceptions without an error code.
/// Layout reflects the stack after push_regs!() fires:
///   r15..rax  (pushed by push_regs, low → high address)
///   rip / cs / rflags / rsp / ss  (pushed by CPU)
#[repr(C)]
struct InterruptFrame {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rbp: u64,
    rdi: u64,
    rsi: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,
    // CPU-pushed
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

/// Same as `InterruptFrame` but with an error code between the saved regs and the CPU frame.
#[repr(C)]
struct InterruptFrameWithError {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rbp: u64,
    rdi: u64,
    rsi: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,
    // CPU-pushed
    error_code: u64,
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

macro_rules! push_regs {
    () => {
        "push rax; push rbx; push rcx; push rdx;
         push rsi; push rdi; push rbp;
         push r8; push r9; push r10; push r11;
         push r12; push r13; push r14; push r15;"
    };
}

macro_rules! pop_regs {
    () => {
        "pop r15; pop r14; pop r13; pop r12;
         pop r11; pop r10; pop r9; pop r8;
         pop rbp; pop rdi; pop rsi;
         pop rdx; pop rcx; pop rbx; pop rax;"
    };
}

macro_rules! exception_no_error {
    ($name:ident, $msg:expr) => {
        paste::paste! {
            extern "C" fn [<$name _inner>](frame: *const InterruptFrame) {
                let f = unsafe { &*frame };
                crate::kprintln!(
                    concat!("\n--- CPU EXCEPTION: ", $msg, " ---\n",
                            "  RIP={:#018x}  CS={:#06x}  RFLAGS={:#018x}\n",
                            "  RSP={:#018x}  SS={:#06x}\n",
                            "  RAX={:#018x}  RBX={:#018x}  RCX={:#018x}  RDX={:#018x}\n",
                            "  RSI={:#018x}  RDI={:#018x}  RBP={:#018x}\n",
                            "  R8 ={:#018x}  R9 ={:#018x}  R10={:#018x}  R11={:#018x}\n",
                            "  R12={:#018x}  R13={:#018x}  R14={:#018x}  R15={:#018x}"),
                    f.rip, f.cs, f.rflags,
                    f.rsp, f.ss,
                    f.rax, f.rbx, f.rcx, f.rdx,
                    f.rsi, f.rdi, f.rbp,
                    f.r8, f.r9, f.r10, f.r11,
                    f.r12, f.r13, f.r14, f.r15,
                );
            }

            #[unsafe(naked)]
            extern "C" fn $name() {
                core::arch::naked_asm!(
                    push_regs!(),
                    "mov rdi, rsp",
                    "call {inner}",
                    pop_regs!(),
                    "iretq",
                    inner = sym [<$name _inner>],
                );
            }
        }
    };
}

macro_rules! exception_with_error {
    ($name:ident, $msg:expr) => {
        paste::paste! {
            extern "C" fn [<$name _inner>](frame: *const InterruptFrameWithError) {
                let f = unsafe { &*frame };
                crate::kprintln!(
                    concat!("\n--- CPU EXCEPTION: ", $msg, " ---\n",
                            "  ERROR CODE={:#018x}\n",
                            "  RIP={:#018x}  CS={:#06x}  RFLAGS={:#018x}\n",
                            "  RSP={:#018x}  SS={:#06x}\n",
                            "  RAX={:#018x}  RBX={:#018x}  RCX={:#018x}  RDX={:#018x}\n",
                            "  RSI={:#018x}  RDI={:#018x}  RBP={:#018x}\n",
                            "  R8 ={:#018x}  R9 ={:#018x}  R10={:#018x}  R11={:#018x}\n",
                            "  R12={:#018x}  R13={:#018x}  R14={:#018x}  R15={:#018x}"),
                    f.error_code,
                    f.rip, f.cs, f.rflags,
                    f.rsp, f.ss,
                    f.rax, f.rbx, f.rcx, f.rdx,
                    f.rsi, f.rdi, f.rbp,
                    f.r8, f.r9, f.r10, f.r11,
                    f.r12, f.r13, f.r14, f.r15,
                );
            }

            #[unsafe(naked)]
            extern "C" fn $name() {
                core::arch::naked_asm!(
                    push_regs!(),
                    "mov rdi, rsp",
                    "call {inner}",
                    pop_regs!(),
                    "add rsp, 8", // pop error code
                    "iretq",
                    inner = sym [<$name _inner>],
                );
            }
        }
    };
}

extern "C" fn irq_common_handler(irq: u8) {
    /*match irq {
        0 => {
            kprintln!("Timer interrupt");
        }
        1 => {
            let scancode = crate::arch::x86_64::inb(0x60);
            kprintln!("Keyboard interrupt (scancode: {:#04x})", scancode);
        }
        12 => {
            kprintln!("Mouse interrupt");
        }
        _ => {
            kprintln!("Received IRQ {}", irq);
        }
    }*/

    send_eoi(irq);
}

macro_rules! irq_handler {
    ($name:ident, $irq:expr) => {
        #[unsafe(naked)]
        extern "C" fn $name() {
            core::arch::naked_asm!(
                push_regs!(),
                "mov rdi, {irq}",
                "call {handler}",
                pop_regs!(),
                "iretq",
                irq = const $irq,
                handler = sym irq_common_handler,
            );
        }
    };
}

exception_no_error!(divide_error, "Exception: Divide Error");
exception_no_error!(debug, "Exception: Debug");
exception_no_error!(nmi, "Exception: NMI");
exception_no_error!(breakpoint, "Exception: Breakpoint");
exception_no_error!(overflow, "Exception: Overflow");
exception_no_error!(bound_range, "Exception: Bound Range");
exception_no_error!(invalid_opcode, "Exception: Invalid Opcode");
exception_no_error!(device_not_available, "Exception: Device Not Available");
exception_no_error!(x87_fp_exception, "Exception: x87 FP Exception");
exception_no_error!(simd_fp_exception, "Exception: SIMD FP Exception");
exception_no_error!(virtualization, "Exception: Virtualization Exception");
exception_no_error!(machine_check, "Exception: Machine Check");

exception_with_error!(double_fault, "Exception: Double Fault");
exception_with_error!(invalid_tss, "Exception: Invalid TSS");
exception_with_error!(general_protection, "Exception: GP Fault");
exception_with_error!(page_fault, "Exception: Page Fault");
exception_with_error!(segment_not_present, "Exception: Segment Not Present");
exception_with_error!(stack_segment, "Exception: Stack Segment");
exception_with_error!(alignment_check, "Exception: Alignment Check");

irq_handler!(irq0, 0u8);
irq_handler!(irq1, 1u8);
irq_handler!(irq2, 2u8);
irq_handler!(irq3, 3u8);
irq_handler!(irq4, 4u8);
irq_handler!(irq5, 5u8);
irq_handler!(irq6, 6u8);
irq_handler!(irq7, 7u8);
irq_handler!(irq8, 8u8);
irq_handler!(irq9, 9u8);
irq_handler!(irq10, 10u8);
irq_handler!(irq11, 11u8);
irq_handler!(irq12, 12u8);
irq_handler!(irq13, 13u8);
irq_handler!(irq14, 14u8);
irq_handler!(irq15, 15u8);

#[unsafe(naked)]
extern "C" fn syscall_handler() {
    core::arch::naked_asm!(
        push_regs!(),
        // TODO: dispatch syscall
        pop_regs!(),
        "iretq",
    );
}

pub fn init() {
    unsafe {
        // CPU exceptions (0-31)
        IDT.entries[0].set_handler(divide_error as *const () as u64);
        IDT.entries[1].set_handler(debug as *const () as u64);
        IDT.entries[2].set_handler(nmi as *const () as u64);
        IDT.entries[3].set_handler(breakpoint as *const () as u64);
        IDT.entries[4].set_handler(overflow as *const () as u64);
        IDT.entries[5].set_handler(bound_range as *const () as u64);
        IDT.entries[6].set_handler(invalid_opcode as *const () as u64);
        IDT.entries[7].set_handler(device_not_available as *const () as u64);
        IDT.entries[8] = IdtEntry::new(
            double_fault as *const () as u64,
            KERNEL_CODE_SELECTOR,
            1,
            GateType::Interrupt,
            0,
        );
        IDT.entries[10].set_handler(invalid_tss as *const () as u64);
        IDT.entries[11].set_handler(segment_not_present as *const () as u64);
        IDT.entries[12].set_handler(stack_segment as *const () as u64);
        IDT.entries[13].set_handler(general_protection as *const () as u64);
        IDT.entries[14].set_handler(page_fault as *const () as u64);
        IDT.entries[16].set_handler(x87_fp_exception as *const () as u64);
        IDT.entries[17].set_handler(alignment_check as *const () as u64);
        IDT.entries[18].set_handler(machine_check as *const () as u64);
        IDT.entries[19].set_handler(simd_fp_exception as *const () as u64);
        IDT.entries[20].set_handler(virtualization as *const () as u64);

        // IRQs (32-47)
        IDT.entries[32].set_handler(irq0 as *const () as u64); // Timer
        IDT.entries[33].set_handler(irq1 as *const () as u64); // Keyboard
        IDT.entries[34].set_handler(irq2 as *const () as u64);
        IDT.entries[35].set_handler(irq3 as *const () as u64);
        IDT.entries[36].set_handler(irq4 as *const () as u64);
        IDT.entries[37].set_handler(irq5 as *const () as u64);
        IDT.entries[38].set_handler(irq6 as *const () as u64);
        IDT.entries[39].set_handler(irq7 as *const () as u64);
        IDT.entries[40].set_handler(irq8 as *const () as u64);
        IDT.entries[41].set_handler(irq9 as *const () as u64);
        IDT.entries[42].set_handler(irq10 as *const () as u64);
        IDT.entries[43].set_handler(irq11 as *const () as u64);
        IDT.entries[44].set_handler(irq12 as *const () as u64);
        IDT.entries[45].set_handler(irq13 as *const () as u64);
        IDT.entries[46].set_handler(irq14 as *const () as u64);
        IDT.entries[47].set_handler(irq15 as *const () as u64);

        // Syscall interrupt
        IDT.entries[0x80] = IdtEntry::new(
            syscall_handler as *const () as u64,
            KERNEL_CODE_SELECTOR,
            0,
            GateType::Trap,
            3,
        );

        // Load IDT
        let idt_descriptor = IdtDescriptor {
            size: (size_of::<Idt>() - 1) as u16,
            offset: &IDT as *const _ as u64,
        };

        core::arch::asm!(
            "lidt [{}]",
            in(reg) &idt_descriptor,
            options(nostack)
        );

        init_pic();
    }
}

/// Initialize PIC (Programmable Interrupt Controller)
/// This remaps the PIC's IRQs to interrupts 32-47, which avoids conflicts with CPU exceptions
/// (0-31).
fn init_pic() {
    use crate::arch::x86_64::{inb, outb};

    const PIC1_CMD: u16 = 0x20;
    const PIC1_DATA: u16 = 0x21;
    const PIC2_CMD: u16 = 0xA0;
    const PIC2_DATA: u16 = 0xA1;

    // Save masks
    let _mask1 = inb(PIC1_DATA);
    let _mask2 = inb(PIC2_DATA);

    // ICW1: Initialize + ICW4 needed
    outb(PIC1_CMD, 0x11);
    outb(PIC2_CMD, 0x11);

    // ICW2: Vector offset
    outb(PIC1_DATA, 0x20); // IRQs 0-7 -> interrupts 32-39
    outb(PIC2_DATA, 0x28); // IRQs 8-15 -> interrupts 40-47

    // ICW3: Cascade identity
    outb(PIC1_DATA, 0x04); // IRQ2 has slave
    outb(PIC2_DATA, 0x02); // Slave identity

    // ICW4: 8086 mode
    outb(PIC1_DATA, 0x01);
    outb(PIC2_DATA, 0x01);

    // Restore masks (enable all for now)
    outb(PIC1_DATA, 0x00);
    outb(PIC2_DATA, 0x00);
}

pub fn send_eoi(irq: u8) {
    use crate::arch::x86_64::outb;

    const PIC1_CMD: u16 = 0x20;
    const PIC2_CMD: u16 = 0xA0;

    if irq >= 8 {
        outb(PIC2_CMD, 0x20);
    }
    outb(PIC1_CMD, 0x20);
}
