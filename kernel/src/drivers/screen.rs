use crate::BootInfo;
use derivative::Derivative;
use spin::Mutex;

use alloc::vec::Vec;

// TODO: Support more than default RGB
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Screen {
    address: usize,

    #[derivative(Debug = "ignore")]
    buffer: Vec<u8>,

    // metadata
    pub width: u32,
    pub height: u32,

    pub bits_per_pixel: u8,
    pub stride: u32,

    pub red_shift: u8,
    pub green_shift: u8,
    pub blue_shift: u8,
    pub red_mask: u8,
    pub green_mask: u8,
    pub blue_mask: u8,
}

impl Screen {
    pub const fn new() -> Self {
        Self {
            address: 0,
            buffer: Vec::new(),
            width: 0,
            height: 0,
            bits_per_pixel: 0,
            stride: 0,
            red_shift: 0,
            green_shift: 0,
            blue_shift: 0,
            red_mask: 0,
            green_mask: 0,
            blue_mask: 0,
        }
    }

    pub fn init(&mut self, boot_info: &BootInfo) {
        let info = boot_info.framebuffer;
        let address = info.address as usize;

        self.address = address;

        // calculate new buffer size
        let buffer_size = (info.width as usize) * (info.height as usize) * (info.bpp as usize) / 8;
        self.buffer.resize(buffer_size, 0);

        self.width = info.width;
        self.height = info.height;

        self.bits_per_pixel = info.bpp;
        self.stride = info.pitch;

        self.red_shift = info.red_shift;
        self.green_shift = info.green_shift;
        self.blue_shift = info.blue_shift;

        self.red_mask = info.red_mask;
        self.green_mask = info.green_mask;
        self.blue_mask = info.blue_mask;

        log::debug!(
            "Screen initialized! RGB{}{}{} in use",
            self.red_mask,
            self.green_mask,
            self.blue_mask,
        );
    }

    pub fn sync(&self) {
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.buffer.as_ptr(),
                self.address as *mut u8,
                self.buffer.len(),
            );
        }
    }

    pub fn get_buffer(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    pub fn write(&mut self, data: &[u8]) {
        let buffer = self.get_buffer();
        let len = data.len().min(buffer.len());

        buffer[..len].copy_from_slice(&data[..len]);
    }
}

pub static SCREEN: Mutex<Screen> = Mutex::new(Screen::new());

pub fn init(boot_info: &BootInfo) {
    let mut screen = SCREEN.lock();
    screen.init(boot_info);
}

pub fn sync() {
    let screen = SCREEN.lock();
    screen.sync();
}

pub fn write(data: &[u8]) {
    let mut screen = SCREEN.lock();
    screen.write(data);
}

pub fn get_buffer() -> spin::MutexGuard<'static, Screen> {
    SCREEN.lock()
}

pub fn get_info() -> (u32, u32) {
    let screen = SCREEN.lock();
    (screen.width, screen.height)
}
