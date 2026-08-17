//! Process orchestration: cancellation, event streams, task queues, and sessions.
//!
//! These primitives are FS-agnostic so the runtime and extension crates can share
//! the same transfer UX. Concrete path tasks and filesystem I/O live in `dhara_storage`.

mod cancel;
mod error;
mod events;
mod process_session;
mod queue;
mod reporter;
mod throttle;

pub use cancel::StorageCancellationToken;
pub use error::ProcessError;
pub use events::StorageProcessEvent;
pub use process_session::ProcessSession;
pub use queue::{DEFAULT_TASK_QUEUE_CAPACITY, TaskQueue, TaskQueueReceiver, TaskQueueSender};
pub use reporter::{ProcessEventReporter, SharedProcessEventReporter};
pub use throttle::{BytesEventThrottle, DEFAULT_BYTES_EVENT_DELTA, DEFAULT_BYTES_EVENT_INTERVAL};
