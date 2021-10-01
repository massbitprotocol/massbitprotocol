use parking_lot::{Mutex, RwLock};
use std::time::{Duration, Instant};

lazy_static::lazy_static! {
    /// If an instrumented lock is contended for longer than the specified duration, a warning will
    /// be logged. Environment variable specified in milliseconds. Defaults to 100ms.
    static ref LOCK_CONTENTION_LOG_THRESHOLD: Duration = {
        Duration::from_millis(
            std::env::var("GRAPH_LOCK_CONTENTION_LOG_THRESHOLD_MS")
                .unwrap_or_else(|_| "100".to_string())
                .parse::<u64>()
                .expect("Invalid value for LOCK_CONTENTION_LOG_THRESHOLD_MS environment variable")
       )
    };
}

/// Adds instrumentation for timing the performance of the lock.
pub struct TimedRwLock<T> {
    id: String,
    lock: RwLock<T>,
    log_threshold: Duration,
}

impl<T> TimedRwLock<T> {
    pub fn new(x: T, id: impl Into<String>) -> Self {
        TimedRwLock {
            id: id.into(),
            lock: RwLock::new(x),
            log_threshold: *LOCK_CONTENTION_LOG_THRESHOLD,
        }
    }

    pub fn write(&self) -> parking_lot::RwLockWriteGuard<T> {
        let start = Instant::now();
        let guard = self.lock.write();
        let elapsed = start.elapsed();
        if elapsed > self.log_threshold {}
        guard
    }

    pub fn read(&self) -> parking_lot::RwLockReadGuard<T> {
        let start = Instant::now();
        let guard = self.lock.read();
        let elapsed = start.elapsed();
        if elapsed > self.log_threshold {}
        guard
    }
}

/// Adds instrumentation for timing the performance of the lock.
pub struct TimedMutex<T> {
    id: String,
    lock: Mutex<T>,
    log_threshold: Duration,
}

impl<T> TimedMutex<T> {
    pub fn new(x: T, id: impl Into<String>) -> Self {
        TimedMutex {
            id: id.into(),
            lock: Mutex::new(x),
            log_threshold: *LOCK_CONTENTION_LOG_THRESHOLD,
        }
    }

    pub fn lock(&self) -> parking_lot::MutexGuard<T> {
        let start = Instant::now();
        let guard = self.lock.lock();
        let elapsed = start.elapsed();
        if elapsed > self.log_threshold {}
        guard
    }
}
