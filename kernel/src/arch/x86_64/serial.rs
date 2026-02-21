use core::fmt::Write;

use crate::arch::x86_64::{inb, outb};

// Port base

const COM1: u16 = 0x3F8;

// Register offsets from the port base
//
// When DLAB=0 (normal mode), offsets 0 and 1 are:
const REG_DATA: u16 = 0; // Receive Buffer / Transmit Holding Register
const REG_IER: u16 = 1; // Interrupt Enable Register
// When DLAB=1, offsets 0 and 1 address the baud-rate divisor instead:
const REG_BAUD_LO: u16 = 0; // Divisor Latch, low byte
const REG_BAUD_HI: u16 = 1; // Divisor Latch, high byte
// Always accessible regardless of DLAB:
const REG_FCR: u16 = 2; // FIFO Control Register
const REG_LCR: u16 = 3; // Line Control Register
const REG_MCR: u16 = 4; // Modem Control Register
const REG_LSR: u16 = 5; // Line Status Register

// Register flag values

const LCR_8N1: u8 = 0x03; // 8 data bits, no parity, 1 stop bit
const LCR_DLAB: u8 = 0x80; // Divisor Latch Access Bit — gates baud registers

const FCR_ENABLE_14B: u8 = 0xC7; // Enable FIFO, clear Tx/Rx, 14-byte threshold

const MCR_LOOPBACK: u8 = 0x1E; // RTS + OUT1 + OUT2 + LOOP (bit 4 enables loopback)
const MCR_NORMAL: u8 = 0x0F; // DTR + RTS + OUT1 + OUT2  (LOOP bit cleared)

const LSR_DATA_READY: u8 = 0x01; // Bit 0: received data is available
const LSR_THR_EMPTY: u8 = 0x20; // Bit 5: transmit-hold register is empty

// Misc

/// Baud divisor for 115200: `clock (1.8432 MHz) / (16 × 115200) = 1`.
const BAUD_115200: (u8, u8) = (0x01, 0x00); // (low byte, high byte)

const LOOPBACK_TEST_BYTE: u8 = 0xAE;

// Implementation

pub struct Serial {
    port: u16,
}

impl Serial {
    pub const fn new(port: u16) -> Self {
        Serial { port }
    }

    /// Initialize the port at 115200 baud, 8N1, no interrupts.
    /// Panics if the loopback self-test fails.
    pub fn init(&self) {
        self.disable_interrupts();
        self.set_baud(BAUD_115200);
        self.configure_line(LCR_8N1);
        self.configure_fifo(FCR_ENABLE_14B);
        self.loopback_test();
    }

    fn reg(&self, offset: u16) -> u16 {
        self.port + offset
    }

    fn disable_interrupts(&self) {
        outb(self.reg(REG_IER), 0x00);
    }

    /// Set baud rate via the divisor latch. `divisor` is `(low_byte, high_byte)`.
    fn set_baud(&self, divisor: (u8, u8)) {
        outb(self.reg(REG_LCR), LCR_DLAB); // Enable divisor latch
        outb(self.reg(REG_BAUD_LO), divisor.0);
        outb(self.reg(REG_BAUD_HI), divisor.1);
        // Writing LCR without DLAB clears it, restoring REG_DATA / REG_IER
    }

    fn configure_line(&self, lcr: u8) {
        outb(self.reg(REG_LCR), lcr);
    }

    fn configure_fifo(&self, fcr: u8) {
        outb(self.reg(REG_FCR), fcr);
    }

    /// Enable loopback mode, write a test byte, read it back, then restore normal mode.
    fn loopback_test(&self) {
        outb(self.reg(REG_MCR), MCR_LOOPBACK);
        outb(self.reg(REG_DATA), LOOPBACK_TEST_BYTE);

        let result = inb(self.reg(REG_DATA));
        if result != LOOPBACK_TEST_BYTE {
            panic!(
                "Serial self-test failed: wrote 0x{:02X}, read 0x{:02X}",
                LOOPBACK_TEST_BYTE, result
            );
        }

        outb(self.reg(REG_MCR), MCR_NORMAL);
    }

    pub fn write_byte(&self, byte: u8) {
        while inb(self.reg(REG_LSR)) & LSR_THR_EMPTY == 0 {}
        outb(self.reg(REG_DATA), byte);
    }

    pub fn read_byte(&self) -> Option<u8> {
        if inb(self.reg(REG_LSR)) & LSR_DATA_READY != 0 {
            Some(inb(self.reg(REG_DATA)))
        } else {
            None
        }
    }

    pub fn write_string(&self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }
}

impl Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

impl Default for Serial {
    fn default() -> Self {
        Self::new(COM1)
    }
}

impl core::fmt::Debug for Serial {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Serial")
            .field("port", &format_args!("0x{:04X}", self.port))
            .finish()
    }
}
