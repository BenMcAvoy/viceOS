pub mod keyboard;
pub mod screen;

use crate::BootInfo;

pub fn init(boot_info: &BootInfo) {
    keyboard::init();
    screen::init(boot_info);

    log::info!("Drivers initialized");
}
