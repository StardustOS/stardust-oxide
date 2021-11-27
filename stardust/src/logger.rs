//! Logger implementation

use {
    log::{Level, LevelFilter, Log, Metadata, Record},
    xen::{console::Writer, println},
};

static LOGGER: Logger = Logger;

/// Initialise logger using the xen::console backend
pub fn init() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .expect("Failed to set logger");
}

struct Logger;

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        println!("{} {}", format_level(record.level()), record.args());
    }

    fn flush(&self) {
        Writer::flush();
    }
}

fn format_level(level: Level) -> &'static str {
    match level {
        Level::Trace => "\x1b[0;35mTRACE\x1b[0m",
        Level::Debug => "\x1b[0;34mDEBUG\x1b[0m",
        Level::Info => "\x1b[0;32mINFO \x1b[0m",
        Level::Warn => "\x1b[0;33mWARN \x1b[0m",
        Level::Error => "\x1b[0;31mERROR\x1b[0m",
    }
}
