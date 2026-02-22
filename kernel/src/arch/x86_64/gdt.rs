//! The GDT (Global Descriptor Table) is a data structure used by x86 processors to define the
//! characteristics of the various memory segments used in protected mode. It contains entries that
//! describe the base address, limit, and access rights of each segment. The GDT is essential for
//! memory management and protection in modern operating systems.
//!
//! Honestly, in a 64-bit OS, the GDT is mostly a formality, as segmentation is largely unused.
//! However, we still need to set up a minimal GDT to enter long mode and ensure that the CPU is in
//! a known state.
//!
//! The required segments are the null segment (which is unused but must be present), a code
//! segment, and a data segment. The TSS (Task State Segment) is also required for handling
//! interrupts and exceptions, but it is not used for task switching in modern operating systems.

use core::mem::size_of;

use crate::kprintln;

#[repr(C, packed)]
struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl GdtEntry {
    const fn new(base: u32, limit: u32, access: u8, granularity: u8) -> Self {
        GdtEntry {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_mid: ((base >> 16) & 0xFF) as u8,
            access,
            granularity: ((limit >> 16) & 0x0F) as u8 | (granularity & 0xF0),
            base_high: ((base >> 24) & 0xFF) as u8,
        }
    }

    const fn code() -> Self {
        Self::new(0, 0xFFFFF, 0x9A, 0xA0) // Code segment: present, ring 0, executable, readable
    }

    const fn data() -> Self {
        Self::new(0, 0xFFFFF, 0x92, 0xA0) // Data segment: present, ring 0, writable
    }

    const fn user_code() -> Self {
        Self::new(0, 0xFFFFF, 0xFA, 0xA0) // User code segment: present, ring 3, executable, readable
    }

    const fn user_data() -> Self {
        Self::new(0, 0xFFFFF, 0xF2, 0xA0) // User data segment: present, ring 3, writable
    }

    const fn null() -> Self {
        Self::new(0, 0, 0, 0) // Null segment: required, but unused
    }
}

// The TssEntry is 16 bytes in 64-bit mode, it's equivalent to 2 GdtEntry slots
// The Tss is needed for handling interrupts and exceptions, but it is not used for task switching
// in modern operating systems. It supplies a stack pointer for when the CPU transitions from user
// mode to kernel mode or when an interrupt occurs. (e.g. if a double fault occurs, a safe stack is
// needed to handle it)
#[repr(C, packed)]
struct TssEntry {
    length: u16,
    base_low: u16,
    base_mid: u8,
    flags1: u8,
    flags2: u8,
    base_high: u8,
    base_upper: u32,
    reserved: u32,
}

impl TssEntry {
    const fn null() -> Self {
        Self {
            length: 0,
            base_low: 0,
            base_mid: 0,
            flags1: 0,
            flags2: 0,
            base_high: 0,
            base_upper: 0,
            reserved: 0,
        }
    }

    fn new(tss_addr: u64, tss_size: u16) -> Self {
        Self {
            length: tss_size,
            base_low: (tss_addr & 0xFFFF) as u16,
            base_mid: ((tss_addr >> 16) & 0xFF) as u8,
            flags1: 0x89, // Present, ring 0, type 9 (available 64-bit TSS)
            flags2: 0,
            base_high: ((tss_addr >> 24) & 0xFF) as u8,
            base_upper: (tss_addr >> 32) as u32,
            reserved: 0,
        }
    }
}

// Task State Segment (TSS)
// These belong in the GDT, but they take up 2 entries, so we need to define them separately
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved1: u32,
    rsps: [u64; 3],
    reserved2: u64,
    ists: [u64; 7],
    reserved3: u64,
    reserved4: u16,
    io_map_base: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        Self {
            reserved1: 0,
            rsps: [0; 3],
            reserved2: 0,
            ists: [0; 7],
            reserved3: 0,
            reserved4: 0,
            io_map_base: size_of::<Self>() as u16,
        }
    }
}

#[repr(C, packed)]
struct GdtDescriptor {
    limit: u16,
    base: u64,
}

#[repr(C)]
pub struct Gdt {
    null: GdtEntry,        // Null segment (required, but unused)
    kernel_code: GdtEntry, // Kernel code segment
    kernel_data: GdtEntry, // Kernel data segment
    user_code: GdtEntry,   // User code segment
    user_data: GdtEntry,   // User data segment
    tss_entry: TssEntry,   // TSS takes up 2 entries
}

static mut GDT: Gdt = Gdt {
    null: GdtEntry::null(),
    kernel_code: GdtEntry::code(),
    kernel_data: GdtEntry::data(),
    user_code: GdtEntry::user_code(),
    user_data: GdtEntry::user_data(),
    tss_entry: TssEntry::null(), // Will be initialized later
};

static mut TSS: TaskStateSegment = TaskStateSegment::new();

/// Kernel stack for syscalls and interrupts
static mut KERNEL_STACK: [u8; 32768] = [0; 32768]; // 32KB, used for kernel mode stack during syscalls and interrupts
static mut IST_STACK0: [u8; 16384] = [0; 16384]; // Used for double faults and stuff

/// Segment selectors
pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
pub const USER_CODE_SELECTOR: u16 = 0x18 | 3;
pub const USER_DATA_SELECTOR: u16 = 0x20 | 3;
pub const TSS_SELECTOR: u16 = 0x28;

pub fn init() {
    kprintln!("Initializing GDT...");

    unsafe {
        let tss_addr = &TSS as *const _ as u64;

        // TSS limit is size - 1 due to
        // indexing starting at 0 (CPU expects this in indexing)
        let tss_size = (size_of::<TaskStateSegment>() - 1) as u16;

        // Set kernel SP
        TSS.rsps[0] = (&KERNEL_STACK[KERNEL_STACK.len() - 1] as *const u8) as u64;
        TSS.ists[0] = (&IST_STACK0[IST_STACK0.len() - 1] as *const u8) as u64;

        // Set TSS entry in GDT
        GDT.tss_entry = TssEntry::new(tss_addr, tss_size);

        kprintln!(
            "GDT initialized with TSS at {:#x}, size {:#x}",
            tss_addr,
            tss_size
        );

        // Create GDT descriptor (used for lgdt instruction)
        let gdt_descriptor = GdtDescriptor {
            limit: (size_of::<Gdt>() - 1) as u16,
            base: &GDT as *const _ as u64,
        };

        kprintln!("Loading GDT....");

        // Load GDT using lgdt instruction
        load_gdt(&gdt_descriptor);

        kprintln!("GDT loaded, reloading segment registers...");

        // Reload segment registers to use new GDT entries
        reload_segments();

        kprintln!("Segment registers reloaded, loading TSS...");

        // Load TSS using ltr instruction
        load_tss(TSS_SELECTOR);

        kprintln!("TSS loaded, GDT initialization complete");
    }
}

// helper functions
fn load_gdt(gdt_descriptor: &GdtDescriptor) {
    unsafe {
        core::arch::asm!(
            "lgdt [{0}]",
            in(reg) gdt_descriptor,
            options(nostack, preserves_flags)
        );
    }
}

/// Reload segment registers
/// This is needed after loading the GDT to ensure that the CPU uses the new segment descriptors.
fn reload_segments() {
    unsafe {
        core::arch::asm!(
            // Push CS selector then the address of the label so retfq
            // jumps there with the new code segment in effect.
            "push rax",            // Push code selector
            "lea rcx, [rip + 2f]", // Get address of label (use explicit scratch reg)
            "push rcx",            // Push return address
            "retfq",               // Far return — flushes CS pipeline
            "2:",
            "mov ds, dx",          // Reload data segments
            "mov es, dx",
            "mov fs, dx",
            "mov gs, dx",
            "mov ss, dx",
            // Use explicit registers so the compiler cannot alias the scratch
            // register (rcx) with the data-selector register (rdx).
            in("rax") KERNEL_CODE_SELECTOR as u64,
            in("rdx") KERNEL_DATA_SELECTOR as u64,
            out("rcx") _,
            options(nostack)
        );
    }
}

/// Load TSS
fn load_tss(selector: u16) {
    unsafe {
        core::arch::asm!(
            "ltr {0:x}",
            in(reg) selector,
            options(nostack)
        );
    }
}

/// Get TSS mutable reference (safe wrapper around unsafe static mutable reference)
pub fn get_tss() -> &'static mut TaskStateSegment {
    unsafe { &mut TSS }
}
