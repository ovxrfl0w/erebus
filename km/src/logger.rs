use wdk::println as wprintln;

#[allow(dead_code)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl LogLevel {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "[i]",
            Self::Success => "[s]",
            Self::Warning => "[w]",
            Self::Error => "[e]",
        }
    }
}

pub struct Logger {}

impl Logger {
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn log(msg: &str, level: LogLevel) {
        wprintln!("[erebus] {} -> {}", level.as_str(), msg);
    }
}

#[macro_export]
macro_rules! println {
    ($msg:tt) => {
        $crate::Logger::log($msg, $crate::LogLevel::Info);
    };
    ($lvl:expr, $msg:tt) => {
        $crate::Logger::log($msg, $lvl);
    };
    ($lvl:expr, $fmt:expr, $($arg:tt)*) => {
        $crate::Logger::log(&format!($fmt, $($arg)*), $lvl);
    };
}
