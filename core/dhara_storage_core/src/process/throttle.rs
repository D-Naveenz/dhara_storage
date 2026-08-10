//! Coalesce frequent byte updates into sparse [`super::StorageProcessEvent::Bytes`] emissions.

use std::time::{Duration, Instant};

/// Default minimum wall time between byte events.
pub const DEFAULT_BYTES_EVENT_INTERVAL: Duration = Duration::from_millis(75);

/// Default minimum transferred delta between byte events.
pub const DEFAULT_BYTES_EVENT_DELTA: u64 = 1024 * 1024;

/// Tracks when cumulative byte progress should be published.
#[derive(Debug, Clone)]
pub struct BytesEventThrottle {
    last_emit_at: Option<Instant>,
    last_emitted_bytes: u64,
    min_interval: Duration,
    min_delta: u64,
}

impl Default for BytesEventThrottle {
    fn default() -> Self {
        Self::new(DEFAULT_BYTES_EVENT_INTERVAL, DEFAULT_BYTES_EVENT_DELTA)
    }
}

impl BytesEventThrottle {
    /// Create a throttle with the given interval and byte-delta thresholds.
    pub fn new(min_interval: Duration, min_delta: u64) -> Self {
        Self {
            last_emit_at: None,
            last_emitted_bytes: 0,
            min_interval,
            min_delta,
        }
    }

    /// Returns true when a [`super::StorageProcessEvent::Bytes`] should be emitted.
    pub fn should_emit(&self, bytes_transferred: u64, now: Instant) -> bool {
        match self.last_emit_at {
            None => true,
            Some(last) => {
                let elapsed = now.saturating_duration_since(last) >= self.min_interval;
                let delta = bytes_transferred.saturating_sub(self.last_emitted_bytes) >= self.min_delta;
                elapsed || delta
            }
        }
    }

    /// Record that a byte event was emitted at `bytes_transferred`.
    pub fn mark_emitted(&mut self, bytes_transferred: u64, now: Instant) {
        self.last_emit_at = Some(now);
        self.last_emitted_bytes = bytes_transferred;
    }

    /// Force the next [`Self::should_emit`] check to succeed (e.g. before item change).
    pub fn force_next(&mut self) {
        self.last_emit_at = None;
    }
}
