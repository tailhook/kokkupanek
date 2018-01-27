use std::cell::RefCell;

use log::{self, Record, Level, Metadata};

static mut LOGGER: GlobalLogger = GlobalLogger {
    cur: None,
};

#[cfg(target_arch="wasm32")]
unsafe impl Sync for GlobalLogger {}

struct GlobalLogger {
    cur: Option<RefCell<Logger>>,
}

struct Logger {
    buf: String,
    sublogger: String,
}

pub struct SchedulerLogger;
pub struct Sublogger(usize);

impl SchedulerLogger {
    pub fn context() -> SchedulerLogger {
        unsafe {
            if LOGGER.cur.is_some() {
                panic!("nested scheduler logging context")
            }
            LOGGER.cur = Some(RefCell::new(Logger {
                buf: String::with_capacity(8096),
                sublogger: String::with_capacity(16),
            }))
        }
        return SchedulerLogger;
    }
    pub fn into_inner(self) -> String {
        let buf = unsafe {
            LOGGER.cur.take().expect("scheduler is set and not nested")
        };
        drop(self);
        return buf.into_inner().buf;
    }
}

impl Drop for SchedulerLogger {
    fn drop(&mut self) {
        unsafe {
            LOGGER.cur.take();
        }
    }
}

impl Sublogger {
    pub fn context(name: &str) -> Sublogger {
        unsafe {
            let mut lg = LOGGER.cur.as_mut()
                .expect("logger is set").borrow_mut();
            let sub = Sublogger(lg.sublogger.len());
            if lg.sublogger.len() != 0 {
                lg.sublogger.push('.');
            }
            lg.sublogger.push_str(name);
            return sub;
        }
    }
}

impl Drop for Sublogger {
    fn drop(&mut self) {
        unsafe {
            let mut lg = LOGGER.cur.as_mut()
                .expect("logger is set").borrow_mut();
            lg.sublogger.truncate(self.0);
        }
    }
}

impl log::Log for GlobalLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        use std::fmt::Write;
        if self.enabled(record.metadata()) {
            if self.cur.is_none() {
                panic!("logging when no schedule is active");
            }
            let ref mut log = *self.cur.as_ref()
                .expect("logger is set").borrow_mut();
            writeln!(&mut log.buf,
                "{:>5}: {}[{}]: {}",
                    record.level(),
                    record.module_path().unwrap_or("<unknown>"),
                    &log.sublogger,
                    record.args())
                    .expect("can write to buffer");
        }
    }

    fn flush(&self) { }
}

pub fn init() {
    unsafe {
        log::set_logger(&LOGGER).expect("log init ok");
    }
}
