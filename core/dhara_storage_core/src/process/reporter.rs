//! Reporter callback for [`super::StorageProcessEvent`] streams.

use std::sync::Arc;

use super::events::StorageProcessEvent;

/// Receives process events from an in-flight transfer session.
pub trait ProcessEventReporter: Send + Sync + 'static {
    /// Receives one process event.
    fn report(&self, event: StorageProcessEvent);
}

impl<F> ProcessEventReporter for F
where
    F: Fn(StorageProcessEvent) + Send + Sync + 'static,
{
    fn report(&self, event: StorageProcessEvent) {
        self(event);
    }
}

/// Shared reporter type used by sync APIs and daemon/FFI bridges.
pub type SharedProcessEventReporter = Arc<dyn ProcessEventReporter>;
