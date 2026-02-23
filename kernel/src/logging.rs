use core::fmt::Write;
use core::sync::atomic::{AtomicU8, Ordering};
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};

#[derive(Default)]
pub struct SerialLogger {
    log_level_int: AtomicU8,
}

// Table of log levels corresponding ANSI colour codes
const LOG_LEVEL_COLOURS: [&str; 6] = [
    "\x1b[0m",  // Off
    "\x1b[31m", // Error
    "\x1b[33m", // Warn
    "\x1b[32m", // Info
    "\x1b[34m", // Debug
    "\x1b[36m", // Trace
];

impl SerialLogger {
    fn get_log_level(&self) -> LevelFilter {
        match self.log_level_int.load(Ordering::SeqCst) {
            0 => LevelFilter::Off,
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            5 => LevelFilter::Trace,
            _ => LevelFilter::Off,
        }
    }

    fn set_log_level(&self, level: LevelFilter) {
        let level_int = match level {
            LevelFilter::Off => 0,
            LevelFilter::Error => 1,
            LevelFilter::Warn => 2,
            LevelFilter::Info => 3,
            LevelFilter::Debug => 4,
            LevelFilter::Trace => 5,
        };
        self.log_level_int.store(level_int, Ordering::SeqCst);
        log::info!("Log level set to {}", level);
    }

    fn get_log_colour(&self, level: Level) -> &str {
        let level_int = level as usize;
        let col = LOG_LEVEL_COLOURS.get(level_int).unwrap_or(&"\x1b[0m");
        col
    }
}

impl log::Log for SerialLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.get_log_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // use SERIAL
        use crate::arch::x86_64::serial::SERIAL;
        let mut ser = SERIAL.lock();
        const RESET_COLOUR: &str = "\x1b[0m";

        let colour = self.get_log_colour(record.level());
        write!(
            ser,
            "{}[{}] - {}: {}{}\n",
            colour,
            record.level(),
            record.target(),
            record.args(),
            RESET_COLOUR
        )
        .unwrap();
    }

    fn flush(&self) {}
}

static LOGGER: SerialLogger = SerialLogger {
    log_level_int: AtomicU8::new(LevelFilter::Info as u8),
};

pub fn init(level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace))?;
    LOGGER.set_log_level(level);

    Ok(())
}
