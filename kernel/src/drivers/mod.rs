pub mod keyboard;
pub mod screen;

use crate::BootInfo;

pub fn init(boot_info: &BootInfo) {
    log::trace!("Initializing drivers...");

    log::trace!("Initializing keyboard driver...");
    keyboard::init();

    log::trace!("Initializing screen driver...");
    screen::init(boot_info);

    log::info!("Drivers initialized");
}
