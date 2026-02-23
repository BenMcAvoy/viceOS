/// Every PTE has flags
/// These flags control how the page is accessed, whether it's present in memory, whether it's
/// writable, etc. This defines the flags for a page table entry (PTE) in x86_64 architecture.
pub mod flags {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER_ACCESSIBLE: u64 = 1 << 2;
    pub const WRITE_THROUGH: u64 = 1 << 3;
    pub const CACHE_DISABLE: u64 = 1 << 4;
    pub const ACCESSED: u64 = 1 << 5;
    pub const DIRTY: u64 = 1 << 6;
    pub const HUGE_PAGE: u64 = 1 << 7;
    pub const GLOBAL: u64 = 1 << 8;
    pub const NO_EXECUTE: u64 = 1 << 63;
}

const ADDR_MASK: u64 = 0x000FFFFFFFFFF000;
const FLAG_MASK: u64 = 0x8000000000000FFF;

/// A page table entry (PTE) is a 64-bit value that contains the physical address of the page and
/// the flags that control how the page is accessed. The structure of a PTE is as follows:
/// - Bits 0-11: Flags (present, writable, user-accessible, etc.)
/// - Bits 12-51: Physical address of the page (must be aligned to 4KB, so the lower 12 bits are
/// always 0)
/// - Bits 52-62: Reserved (must be 0)
/// - Bit 63: No-execute flag (if set, code cannot be executed from this page)
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn new(addr: u64, flags: u64) -> Self {
        Self((addr & ADDR_MASK) | (flags & FLAG_MASK))
    }

    pub fn addr(&self) -> u64 {
        self.0 & ADDR_MASK
    }

    pub fn flags(&self) -> u64 {
        self.0 & FLAG_MASK
    }

    pub fn set_addr(&mut self, addr: u64) {
        self.0 = (self.0 & FLAG_MASK) | (addr & ADDR_MASK);
    }

    pub fn set_flags(&mut self, flags: u64) {
        self.0 = (self.0 & ADDR_MASK) | (flags & FLAG_MASK);
    }

    // Flags helpers
    pub fn is_present(&self) -> bool {
        self.flags() & flags::PRESENT != 0
    }
    pub fn is_writable(&self) -> bool {
        self.flags() & flags::WRITABLE != 0
    }
    pub fn is_user_accessible(&self) -> bool {
        self.flags() & flags::USER_ACCESSIBLE != 0
    }
    pub fn is_write_through(&self) -> bool {
        self.flags() & flags::WRITE_THROUGH != 0
    }
    pub fn is_cache_disabled(&self) -> bool {
        self.flags() & flags::CACHE_DISABLE != 0
    }
    pub fn is_accessed(&self) -> bool {
        self.flags() & flags::ACCESSED != 0
    }
    pub fn is_dirty(&self) -> bool {
        self.flags() & flags::DIRTY != 0
    }
    pub fn is_huge_page(&self) -> bool {
        self.flags() & flags::HUGE_PAGE != 0
    }
    pub fn is_global(&self) -> bool {
        self.flags() & flags::GLOBAL != 0
    }
    pub fn is_no_execute(&self) -> bool {
        self.flags() & flags::NO_EXECUTE != 0
    }
}

// A page table contains one level of the page table hierarchy. In x86_64, there are 4 levels of
// page tables: PML4, PDPT, PD, and PT. Each page table contains 512 entries, and each entry is a
// PageTableEntry.
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    pub const fn empty() -> Self {
        Self {
            entries: [PageTableEntry::empty(); 512],
        }
    }
}

impl core::fmt::Debug for PageTable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PageTable")
            .field("entries", &"512 entries")
            .finish()
    }
}

// accessor for entries
impl core::ops::Index<usize> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl core::ops::IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

pub struct PageTableIndices {
    pub pml4: usize,
    pub pdpt: usize,
    pub pd: usize,
    pub pt: usize,
    pub offset: usize,
}

pub struct VirtualAddress(u64);

impl VirtualAddress {
    pub fn indices(&self) -> PageTableIndices {
        PageTableIndices {
            pml4: ((self.0 >> 39) & 0x1FF) as usize,
            pdpt: ((self.0 >> 30) & 0x1FF) as usize,
            pd: ((self.0 >> 21) & 0x1FF) as usize,
            pt: ((self.0 >> 12) & 0x1FF) as usize,
            offset: (self.0 & 0xFFF) as usize,
        }
    }
}

// TODO: This doesn't look like the standard way to do this, but it works for now. We can change it
// later if we want to use a more standard approach...
// We don' have a PT kernel for some reason??
static mut KPML4: PageTable = PageTable::empty();
static mut KPDPT: PageTable = PageTable::empty();
static mut KPD: [PageTable; 4] = [
    PageTable::empty(),
    PageTable::empty(),
    PageTable::empty(),
    PageTable::empty(),
];

/// Physaddr of the page tables. This is needed to set up the CR3 register, which points to the
/// PML4 table.
static mut PAGE_TABLE_PHYS: u64 = 0;

/// Initialize paging
pub fn init() {
    unsafe {
        let pml4_addr = &KPML4 as *const _ as u64;
        let pdpt_addr = &KPDPT as *const _ as u64;

        // PML4[0] -> PDPT
        KPML4[0] = PageTableEntry::new(pdpt_addr, flags::PRESENT | flags::WRITABLE);
        // PML4[511] -> PDPT (for higher half)
        KPML4[511] = PageTableEntry::new(pdpt_addr, flags::PRESENT | flags::WRITABLE);

        // PDPTR entries, 4 entries for 4GB of memory (each entry maps 1GB)
        for i in 0..4 {
            let pd_addr = &KPD[i] as *const _ as u64;
            KPDPT[i] = PageTableEntry::new(pd_addr, flags::PRESENT | flags::WRITABLE);
        }

        for j in 0..4 {
            for i in 0..512 {
                // PD entries, each entry maps 2MB (512 * 2MB = 1GB)
                KPD[j][i] = PageTableEntry::new(
                    (j as u64 * 512 + i as u64) * 0x200000,
                    flags::PRESENT | flags::WRITABLE | flags::HUGE_PAGE,
                );
            }
        }

        PAGE_TABLE_PHYS = pml4_addr;
        crate::arch::x86_64::write_cr3(PAGE_TABLE_PHYS);
    }
}

/// Map virt -> phys
pub fn map_page(virt: u64, phys: u64, flags: u64) -> Result<(), &'static str> {
    let indices = VirtualAddress(virt).indices();

    unsafe {
        let pml4e = &mut KPML4[indices.pml4];
        if !pml4e.is_present() {
            let pdpt_phys =
                crate::mem::phys::alloc_frame().ok_or("Failed to allocate frame for PDPT")?;
            *pml4e = PageTableEntry::new(pdpt_phys, flags::PRESENT | flags::WRITABLE);

            // Zero the new table
            let pdpt = pml4e.addr() as *mut PageTable;
            core::ptr::write_bytes(pdpt, 0, 1);
        }

        let pdpt = pml4e.addr() as *mut PageTable;
        let pdpte = &mut (*pdpt).entries[indices.pdpt];

        if !pdpte.is_present() {
            let pd_phys =
                crate::mem::phys::alloc_frame().ok_or("Failed to allocate frame for PD")?;
            *pdpte = PageTableEntry::new(pd_phys, flags::PRESENT | flags::WRITABLE);

            // Zero the new table
            let pd = pdpte.addr() as *mut PageTable;
            core::ptr::write_bytes(pd, 0, 1);
        }

        let pd = pdpte.addr() as *mut PageTable;
        let pde = &mut (*pd).entries[indices.pd];

        if !pde.is_present() {
            let pt_phys =
                crate::mem::phys::alloc_frame().ok_or("Failed to allocate frame for PT")?;
            *pde = PageTableEntry::new(pt_phys, flags::PRESENT | flags::WRITABLE);

            // Zero the new table
            let pt = pde.addr() as *mut PageTable;
            core::ptr::write_bytes(pt, 0, 1);
        }

        let pt = pde.addr() as *mut PageTable;
        let pte = &mut (*pt).entries[indices.pt];
        *pte = PageTableEntry::new(phys, flags | flags::PRESENT);

        // Flush TLB to make sure the new mapping is visible to the CPU
        crate::arch::x86_64::invlpg(virt);
    }

    Ok(())
}

fn unmap_page(virt: u64) -> Result<u64, &'static str> {
    let indices = VirtualAddress(virt).indices();

    unsafe {
        let pml4_entry = &mut KPML4[indices.pml4];
        if !pml4_entry.is_present() {
            return Err("PML4 entry not present");
        }

        let pdpt = pml4_entry.addr() as *mut PageTable;
        let pdpt_entry = &(*pdpt).entries[indices.pdpt];
        if !pdpt_entry.is_present() {
            return Err("PDPT entry not present");
        }

        let pd = pdpt_entry.addr() as *mut PageTable;
        let pd_entry = &(*pd).entries[indices.pd];
        if !pd_entry.is_present() {
            return Err("PD entry not present");
        }

        let pt = pd_entry.addr() as *mut PageTable;
        let pt_entry = &mut (*pt).entries[indices.pt];
        if !pt_entry.is_present() {
            return Err("PT entry not present");
        }

        let phys = pt_entry.addr();
        *pt_entry = PageTableEntry::empty();

        crate::arch::x86_64::invlpg(virt);

        Ok(phys)
    }
}

/// Translate virtual address to physical address
pub fn translate(virt: u64) -> Option<u64> {
    let indices = VirtualAddress(virt).indices();

    unsafe {
        let pml4_entry = &KPML4[indices.pml4];
        if !pml4_entry.is_present() {
            return None;
        }

        let pdpt = pml4_entry.addr() as *const PageTable;
        let pdpt_entry = &(*pdpt).entries[indices.pdpt];
        if !pdpt_entry.is_present() {
            return None;
        }

        // Check for 1GB page
        if pdpt_entry.is_huge_page() {
            let phys = pdpt_entry.addr() + (virt & 0x3FFF_FFFF);
            return Some(phys);
        }

        let pd = pdpt_entry.addr() as *const PageTable;
        let pd_entry = &(*pd).entries[indices.pd];
        if !pd_entry.is_present() {
            return None;
        }

        // Check for 2MB page
        if pd_entry.is_huge_page() {
            let phys = pd_entry.addr() + (virt & 0x1F_FFFF);
            return Some(phys);
        }

        let pt = pd_entry.addr() as *const PageTable;
        let pt_entry = &(*pt).entries[indices.pt];
        if !pt_entry.is_present() {
            return None;
        }

        Some(pt_entry.addr() + indices.offset as u64)
    }
}
