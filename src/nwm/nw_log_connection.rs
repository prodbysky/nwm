use colored::Colorize;
use log::Level;
use std::io::Write;
use std::sync::Mutex;

pub struct NwLogLog {
    out: Mutex<std::fs::File>,
}

impl NwLogLog {
    pub fn init(stdin: std::fs::File) -> Self {
        Self {
            out: Mutex::new(stdin),
        }
    }
}

impl log::Log for NwLogLog {
    fn flush(&self) {
        if let Ok(mut stdin) = self.out.lock() {
            let _ = stdin.flush();
        }
    }
    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        if let Ok(mut stdin) = self.out.lock() {
            let _ = writeln!(
                stdin,
                "{} -> {}",
                record.level().as_str().yellow(),
                record.args()
            );
        }
    }

    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Info
    }
}
