use core::fmt::Write;

// inb and outb allow us to read/write to serial ports
use crate::arch::x86_64::{inb, outb};

const COM1: u16 = 0x3F8;

const SERIAL_TEST_BYTE: u8 = 0xAE; // Arbitrary test byte for self-test

const SERIAL_LCR_OFFSET: u16 = 3; // Line Control Register offset

// NOTE: DLAB (Divisor Latch Access Bit) must be set in the Line Control Register (LCR) to access
// the baud rate divisor registers. When DLAB is set, the first two registers (offsets 0 and 1) are
// used for the baud rate divisor instead of data and interrupt enable registers.
const SERIAL_LCR_DLAB: u8 = 0x80; // DLAB bit in LCR
// threshold

const SERIAL_LCR_8N1: u8 = 0x03; // 8 bits, no parity, one stop bit

const SERIAL_INTERUPT_ENABLE_OFFSET: u16 = 1; // Interrupt Enable Register offset

// NOTE: Can only be used after setting DLAB bit in LCR, otherwise this will
// access the data register instead of the baud rate divisor registers (e.g.
// used to set interrupt enable bits in the Interrupt Enable Register)
const SERIAL_BAUD_RATE_DIVISOR_LOW_OFFSET: u16 = 0; // Baud rate divisor low byte offset
const SERIAL_BAUD_RATE_DIVISOR_HIGH_OFFSET: u16 = 1; // Baud rate divisor high byte offset

const SERIAL_FCR_OFFSET: u16 = 2; // FIFO Control Register offset
const SERIAL_FCR_FIFO_14B_THRESHOLD: u8 = 0xC7; // Enable FIFO, clear them, with 14-byte

const SERIAL_DATA_OFFSET: u16 = 0; // Data register offset
const SERIAL_MCR_OFFSET: u16 = 4; // Modem Control Register offset
const SERIAL_LOOPBACK_ENABLE: u8 = 0x1E; // Enable loopback mode: bits 1-4 (RTS, OUT1, OUT2, LOOP)
const SERIAL_LOOPBACK_DISABLE: u8 = 0x0F; // Normal operation: DTR, RTS, OUT1, OUT2 (bit 4/LOOP cleared)

const SERIAL_LSR_OFFSET: u16 = 5; // Line Status Register offset
const SERIAL_LSR_TRANSMIT_MASK: u8 = 0x20; // Bit 5 (0x20) in the Line Status Register indicates if
// the transmit buffer is empty

pub struct Serial {
    // NOTE: We could add fields for baud rate, data bits, etc. if we want to support configuration
    port: u16,
}

impl Serial {
    pub const fn new(port: u16) -> Self {
        Serial { port }
    }

    /// This function initializes the serial port with a standard configuration (115200 baud, 8N1).
    /// It also disables interrupts for the serial port since we'll handle them in the kernel.
    /// This uses `inb` and `outb` to write to the serial port's registers. It will also perform
    /// a self-test by writing to the data register and reading it back. If the test fails, it will
    /// panic.
    pub fn init(&self) {
        // Disable interrupts (we'll handle them in the kernel) (self.port + 1 is the Interrupt
        // Enable Register)
        outb(self.port + SERIAL_INTERUPT_ENABLE_OFFSET, 0x00);

        // Enable DLAB (set baud rate divisor)
        // 0x80 sets the DLAB bit in the Line Control Register (LCR)
        outb(self.port + SERIAL_LCR_OFFSET, SERIAL_LCR_DLAB);

        // Set baud rate divisor to 1 (115200 baud)
        outb(self.port + SERIAL_BAUD_RATE_DIVISOR_LOW_OFFSET, 0x01);
        outb(self.port + SERIAL_BAUD_RATE_DIVISOR_HIGH_OFFSET, 0x00);

        // 0x03 sets 8 bits, no parity, one stop bit (8N1) (NOTE: DLAB is now cleared)
        outb(self.port + SERIAL_LCR_OFFSET, SERIAL_LCR_8N1);

        // Enable FIFO, clear them, with 14-byte threshold
        outb(self.port + SERIAL_FCR_OFFSET, SERIAL_FCR_FIFO_14B_THRESHOLD);

        // Perform loopback test to verify serial port is working
        // Loopback is controlled via bit 4 of the MCR (offset 4), not the data register
        outb(self.port + SERIAL_MCR_OFFSET, SERIAL_LOOPBACK_ENABLE);

        // Write test byte to the data register; in loopback mode it comes back in the RBR
        outb(self.port + SERIAL_DATA_OFFSET, SERIAL_TEST_BYTE);

        // Read back from data register and verify it matches the test byte
        let test_result = inb(self.port + SERIAL_DATA_OFFSET);
        if test_result != SERIAL_TEST_BYTE {
            panic!(
                "Serial port self-test failed: expected 0x{:02X}, got 0x{:02X}",
                SERIAL_TEST_BYTE, test_result
            );
        }

        // Disable loopback, restore normal MCR state
        outb(self.port + SERIAL_MCR_OFFSET, SERIAL_LOOPBACK_DISABLE);
    }

    fn is_transmit_empty(&self) -> bool {
        // The Line Status Register (LSR) is at offset 5, and bit 5 (0x20) indicates if the transmit
        // buffer is empty
        (inb(self.port + SERIAL_LSR_OFFSET) & SERIAL_LSR_TRANSMIT_MASK) != 0
    }

    pub fn write_byte(&self, byte: u8) {
        // Wait until the transmit buffer is empty
        while !self.is_transmit_empty() {}

        // Write the byte to the data register (offset 0)
        outb(self.port + SERIAL_DATA_OFFSET, byte);
    }

    fn has_data(&self) -> bool {
        // Bit 0 (0x01) in the Line Status Register indicates if there is data available to read
        (inb(self.port + SERIAL_LSR_OFFSET) & 0x01) != 0
    }

    pub fn read_byte(&self) -> Option<u8> {
        if self.has_data() {
            Some(inb(self.port + SERIAL_DATA_OFFSET))
        } else {
            None
        }
    }

    pub fn write_string(&self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r'); // Carriage return before newline for proper formatting
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
