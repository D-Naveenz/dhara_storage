//! Broadcast `tracing` events to connected `StreamLogs` gRPC clients.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::proto::LogRecord;

/// Capacity for the in-process log broadcast channel.
const BROADCAST_CAPACITY: usize = 1024;

/// Install the global tracing subscriber and return the broadcast sender for `StreamLogs`.
pub fn init_tracing() -> broadcast::Sender<LogRecord> {
    let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
    let layer = BroadcastLogLayer::new(tx.clone());

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with(layer)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    tx
}

/// Fan out structured log records to all active `StreamLogs` subscribers.
struct BroadcastLogLayer {
    sender: broadcast::Sender<LogRecord>,
}

impl BroadcastLogLayer {
    fn new(sender: broadcast::Sender<LogRecord>) -> Self {
        Self { sender }
    }
}

#[derive(Default)]
struct EventFieldVisitor {
    fields: HashMap<String, String>,
}

impl EventFieldVisitor {
    fn into_record(mut self, metadata: &tracing::Metadata<'_>) -> LogRecord {
        let message = self.fields.remove("message").unwrap_or_default();
        LogRecord {
            level: metadata.level().to_string(),
            target: metadata.target().to_owned(),
            message,
            timestamp_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or_default(),
            module_path: metadata.module_path().map(str::to_owned),
            file: metadata.file().map(str::to_owned),
            line: metadata.line(),
            fields: self.fields,
        }
    }
}

impl tracing::field::Visit for EventFieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields
            .insert(field.name().to_owned(), format!("{value:?}"));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_owned(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_owned(), value.to_string());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_owned(), value.to_string());
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name().to_owned(), value.to_owned());
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.fields
            .insert(field.name().to_owned(), value.to_string());
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields
            .insert(field.name().to_owned(), value.to_string());
    }
}

impl<S> Layer<S> for BroadcastLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = EventFieldVisitor::default();
        event.record(&mut visitor);
        let record = visitor.into_record(event.metadata());
        let _ = self.sender.send(record);
    }
}

/// Map a tracing level string to the `StreamLogsRequest.min_level` scale.
pub fn level_rank(level: &str) -> u32 {
    match level {
        "ERROR" => 1,
        "WARN" => 2,
        "INFO" => 3,
        "DEBUG" => 4,
        "TRACE" => 5,
        _ => 3,
    }
}
