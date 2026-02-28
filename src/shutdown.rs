//! Cross-platform cooperative shutdown signal.
//!
//! [`ShutdownSignal`] allows a controlling thread (e.g. a Windows service
//! handler) to request a graceful stop, while the main polling loop checks
//! the flag between iterations.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Interval used by [`ShutdownSignal::sleep_or_stop`] to check the flag.
const SLEEP_CHUNK: Duration = Duration::from_millis(500);

/// Cooperative shutdown signal backed by an [`AtomicBool`].
///
/// The signal is cheaply cloneable (shared via [`Arc`]) so that one
/// clone can live in a service control handler while another is polled
/// inside the main loop.
#[derive(Clone, Debug)]
pub struct ShutdownSignal {
    flag: Arc<AtomicBool>,
}

impl ShutdownSignal {
    /// Create a new signal with shutdown **not** requested.
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Request a graceful shutdown.
    pub fn request_shutdown(&self) {
        self.flag.store(true, Ordering::Release);
    }

    /// Returns `true` when shutdown has been requested.
    pub fn is_shutdown_requested(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    /// Sleep for `duration`, checking the shutdown flag every 500 ms.
    ///
    /// Returns `true` if shutdown was requested during the wait (i.e. the
    /// caller should stop), `false` if the full duration elapsed normally.
    pub fn sleep_or_stop(&self, duration: Duration) -> bool {
        let mut remaining = duration;
        while remaining > Duration::ZERO {
            if self.is_shutdown_requested() {
                return true;
            }
            let chunk = remaining.min(SLEEP_CHUNK);
            std::thread::sleep(chunk);
            remaining = remaining.saturating_sub(chunk);
        }
        self.is_shutdown_requested()
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_requested() {
        let sig = ShutdownSignal::new();
        assert!(!sig.is_shutdown_requested());
    }

    #[test]
    fn requested_after_request_shutdown() {
        let sig = ShutdownSignal::new();
        sig.request_shutdown();
        assert!(sig.is_shutdown_requested());
    }

    #[test]
    fn clone_shares_state() {
        let sig = ShutdownSignal::new();
        let sig2 = sig.clone();
        sig2.request_shutdown();
        assert!(sig.is_shutdown_requested());
    }

    #[test]
    fn sleep_or_stop_exits_early_on_shutdown() {
        let sig = ShutdownSignal::new();
        let sig2 = sig.clone();
        // Request shutdown from another thread after a short delay.
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            sig2.request_shutdown();
        });
        let start = std::time::Instant::now();
        let stopped = sig.sleep_or_stop(Duration::from_secs(30));
        let elapsed = start.elapsed();
        assert!(stopped);
        // Should have exited well before the 30 s timeout.
        assert!(elapsed < Duration::from_secs(5));
    }

    #[test]
    fn sleep_or_stop_completes_without_shutdown() {
        let sig = ShutdownSignal::new();
        let stopped = sig.sleep_or_stop(Duration::from_millis(50));
        assert!(!stopped);
    }
}
