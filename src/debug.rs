use std::collections::VecDeque;
use std::sync::Mutex;

const MAX_MESSAGES: usize = 200;

static MESSAGES: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

pub fn push(msg: String) {
    if let Ok(mut log) = MESSAGES.lock() {
        if log.len() >= MAX_MESSAGES {
            log.pop_front();
        }
        log.push_back(msg);
    }
}

/// Returns a snapshot of all current debug messages, oldest first.
pub fn snapshot() -> Vec<String> {
    MESSAGES
        .lock()
        .map(|log| log.iter().cloned().collect())
        .unwrap_or_default()
}

#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        $crate::debug::push(format!($($arg)*))
    };
}
