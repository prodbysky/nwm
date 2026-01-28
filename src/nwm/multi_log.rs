use log::Log;

pub struct MultiLog {
    loggers: Vec<Box<dyn Log>>,
}

impl MultiLog {
    pub fn init(loggers: Vec<Box<dyn Log>>, level: log::Level) {
        log::set_max_level(level.to_level_filter());
        log::set_boxed_logger(Box::new(Self { loggers })).unwrap();
    }
}

impl Log for MultiLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.loggers.iter().any(|l| l.enabled(metadata))
    }

    fn log(&self, record: &log::Record) {
        self.loggers.iter().for_each(|l| l.log(record))
    }

    fn flush(&self) {
        self.loggers.iter().for_each(|l| l.flush())
    }
}
