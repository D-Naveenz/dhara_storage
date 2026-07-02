use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::abi::DharaStatus;
use crate::errors::FfiFailure;
use crate::marshal::execute_unit;

type NativeLogCallback = unsafe extern "C" fn(*const u8, usize, *mut c_void);

static CALLBACK: OnceLock<Mutex<Option<CallbackRegistration>>> = OnceLock::new();
static INSTALL_RESULT: OnceLock<Result<(), String>> = OnceLock::new();

#[derive(Clone, Copy)]
struct CallbackRegistration {
    callback: NativeLogCallback,
    user_data: usize,
}

unsafe impl Send for CallbackRegistration {}
unsafe impl Sync for CallbackRegistration {}

#[derive(Debug, Serialize)]
pub(crate) struct NativeLogRecord {
    level: String,
    target: String,
    message: String,
    timestamp_unix_ms: u64,
    module_path: Option<String>,
    file: Option<String>,
    line: Option<u32>,
    fields: BTreeMap<String, String>,
}

#[derive(Default)]
struct EventFieldVisitor {
    fields: BTreeMap<String, String>,
}

impl EventFieldVisitor {
    fn into_record(mut self, metadata: &tracing::Metadata<'_>) -> NativeLogRecord {
        let message = self.fields.remove("message").unwrap_or_default();
        NativeLogRecord {
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

pub(crate) struct NativeLogLayer;

impl<S> Layer<S> for NativeLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let Some(registration) = current_registration() else {
            return;
        };

        let mut visitor = EventFieldVisitor::default();
        event.record(&mut visitor);
        let record = visitor.into_record(event.metadata());
        let Ok(bytes) = serde_json::to_vec(&record) else {
            return;
        };

        unsafe {
            (registration.callback)(
                bytes.as_ptr(),
                bytes.len(),
                registration.user_data as *mut c_void,
            );
        }
    }
}

pub(crate) fn install_native_log_bridge() -> Result<(), FfiFailure> {
    INSTALL_RESULT
        .get_or_init(|| {
            tracing_subscriber::registry()
                .with(NativeLogLayer)
                .try_init()
                .map_err(|error| error.to_string())
        })
        .clone()
        .map_err(FfiFailure::error)
}

pub(crate) fn set_callback(
    callback: NativeLogCallback,
    user_data: *mut c_void,
) -> Result<(), FfiFailure> {
    install_native_log_bridge()?;
    let callback_slot = CALLBACK.get_or_init(|| Mutex::new(None));
    let mut callback_guard = callback_slot
        .lock()
        .map_err(|_| FfiFailure::error("native log callback lock was poisoned"))?;
    *callback_guard = Some(CallbackRegistration {
        callback,
        user_data: user_data as usize,
    });
    Ok(())
}

pub(crate) fn clear_callback() -> Result<(), FfiFailure> {
    let callback_slot = CALLBACK.get_or_init(|| Mutex::new(None));
    let mut callback_guard = callback_slot
        .lock()
        .map_err(|_| FfiFailure::error("native log callback lock was poisoned"))?;
    *callback_guard = None;
    Ok(())
}

fn current_registration() -> Option<CallbackRegistration> {
    CALLBACK
        .get()
        .and_then(|callback_slot| callback_slot.lock().ok().and_then(|guard| *guard))
}

#[unsafe(no_mangle)]
/// Registers a host callback that receives structured native log records as UTF-8 JSON.
///
/// # Arguments
///
/// - `callback` (`Option<unsafe extern "C" fn(*const u8, usize, *mut c_void)>`) - Host callback that receives each serialized log record.
/// - `user_data` (`*mut c_void`) - Opaque pointer passed back to `callback` unchanged.
/// - `out_error_ptr` (`*mut *mut u8`) - Receives an owned UTF-8 error payload when the call fails.
/// - `out_error_len` (`*mut usize`) - Receives the length of the owned error payload in bytes.
///
/// # Returns
///
/// - `DharaStatus` - [`DharaStatus::Ok`] when registration succeeds, otherwise a failure status.
///
/// # Errors
///
/// Returns [`DharaStatus::InvalidArgument`] when `callback` is `None`, or [`DharaStatus::Error`]
/// when the tracing bridge cannot be initialized or the callback registry cannot be updated.
///
/// # Safety
///
/// `out_error_ptr` and `out_error_len` must either both be valid writable pointers or follow the
/// nullability contract expected by the Dhara FFI marshaling helpers. The registered `callback`
/// must remain valid until `dhara_unregister_logger` is called and must not unwind across the FFI boundary.
pub unsafe extern "C" fn dhara_register_logger(
    callback: Option<NativeLogCallback>,
    user_data: *mut c_void,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || {
        let callback = callback.ok_or_else(|| {
            FfiFailure::invalid_argument("callback", "logger callback must not be null")
        })?;
        set_callback(callback, user_data)
    }))
}

#[unsafe(no_mangle)]
/// Unregisters the currently active native log callback.
///
/// # Arguments
///
/// - `out_error_ptr` (`*mut *mut u8`) - Receives an owned UTF-8 error payload when the call fails.
/// - `out_error_len` (`*mut usize`) - Receives the length of the owned error payload in bytes.
///
/// # Returns
///
/// - `DharaStatus` - [`DharaStatus::Ok`] when the callback is cleared successfully.
///
/// # Errors
///
/// Returns [`DharaStatus::Error`] when the callback registry cannot be updated.
///
/// # Safety
///
/// `out_error_ptr` and `out_error_len` must either both be valid writable pointers or follow the
/// nullability contract expected by the Dhara FFI marshaling helpers.
pub unsafe extern "C" fn dhara_unregister_logger(
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, clear_callback))
}
