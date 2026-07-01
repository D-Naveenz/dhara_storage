use std::path::Path;

use serde::Serialize;

use crate::abi::DharaStatus;
use crate::models::path_to_string;
use dhara_storage::StorageError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct ErrorPayload {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) path: Option<String>,
    pub(crate) operation: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) value: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct FfiFailure {
    pub(crate) status: DharaStatus,
    pub(crate) payload: ErrorPayload,
}

impl FfiFailure {
    pub(crate) fn error(message: impl Into<String>) -> Self {
        Self {
            status: DharaStatus::Error,
            payload: ErrorPayload {
                code: "storage_error",
                message: message.into(),
                path: None,
                operation: None,
                kind: None,
                value: None,
            },
        }
    }

    pub(crate) fn io(operation: &'static str, path: &Path, source: std::io::Error) -> Self {
        Self {
            status: DharaStatus::Error,
            payload: ErrorPayload {
                code: "io",
                message: source.to_string(),
                path: Some(path_to_string(path)),
                operation: Some(operation.to_owned()),
                kind: None,
                value: None,
            },
        }
    }

    pub(crate) fn cancelled(operation: &'static str) -> Self {
        Self {
            status: DharaStatus::Error,
            payload: ErrorPayload {
                code: "cancelled",
                message: format!("operation cancelled while attempting to {operation}"),
                path: None,
                operation: Some(operation.to_owned()),
                kind: None,
                value: None,
            },
        }
    }

    pub(crate) fn invalid_argument(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: DharaStatus::InvalidArgument,
            payload: ErrorPayload {
                code: "invalid_argument",
                message: message.into(),
                path: None,
                operation: None,
                kind: Some(field.to_owned()),
                value: None,
            },
        }
    }
}

impl From<StorageError> for FfiFailure {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::NotFound { path } => Self {
                status: DharaStatus::Error,
                payload: ErrorPayload {
                    code: "not_found",
                    message: format!("path does not exist: {}", path_to_string(&path)),
                    path: Some(path_to_string(&path)),
                    operation: None,
                    kind: None,
                    value: None,
                },
            },
            StorageError::AlreadyExists { path } => Self {
                status: DharaStatus::Error,
                payload: ErrorPayload {
                    code: "already_exists",
                    message: format!("path already exists: {}", path_to_string(&path)),
                    path: Some(path_to_string(&path)),
                    operation: None,
                    kind: None,
                    value: None,
                },
            },
            StorageError::NotAFile { path } => Self {
                status: DharaStatus::Error,
                payload: ErrorPayload {
                    code: "not_a_file",
                    message: format!("path is not a file: {}", path_to_string(&path)),
                    path: Some(path_to_string(&path)),
                    operation: None,
                    kind: None,
                    value: None,
                },
            },
            StorageError::NotADirectory { path } => Self {
                status: DharaStatus::Error,
                payload: ErrorPayload {
                    code: "not_a_directory",
                    message: format!("path is not a directory: {}", path_to_string(&path)),
                    path: Some(path_to_string(&path)),
                    operation: None,
                    kind: None,
                    value: None,
                },
            },
            StorageError::InvalidName { kind, value } => Self {
                status: DharaStatus::Error,
                payload: ErrorPayload {
                    code: "invalid_name",
                    message: format!("invalid {kind} name: {value}"),
                    path: None,
                    operation: None,
                    kind: Some(kind.to_owned()),
                    value: Some(value),
                },
            },
            StorageError::PathConflict { path, message } => Self {
                status: DharaStatus::Error,
                payload: ErrorPayload {
                    code: "path_conflict",
                    message: format!("path conflict at '{}': {message}", path_to_string(&path)),
                    path: Some(path_to_string(&path)),
                    operation: None,
                    kind: None,
                    value: None,
                },
            },
            StorageError::Io {
                operation,
                path,
                source,
            } => Self {
                status: DharaStatus::Error,
                payload: ErrorPayload {
                    code: "io",
                    message: source.to_string(),
                    path: Some(path_to_string(&path)),
                    operation: Some(operation.to_owned()),
                    kind: None,
                    value: None,
                },
            },
            StorageError::ReaderIo { operation, source } => Self {
                status: DharaStatus::Error,
                payload: ErrorPayload {
                    code: "reader_io",
                    message: source.to_string(),
                    path: None,
                    operation: Some(operation.to_owned()),
                    kind: None,
                    value: None,
                },
            },
            StorageError::DefinitionsLoad { message } => Self {
                status: DharaStatus::Error,
                payload: ErrorPayload {
                    code: "definitions_load",
                    message,
                    path: None,
                    operation: None,
                    kind: None,
                    value: None,
                },
            },
            StorageError::Cancelled { operation } => Self {
                status: DharaStatus::Error,
                payload: ErrorPayload {
                    code: "cancelled",
                    message: format!("operation cancelled while attempting to {operation}"),
                    path: None,
                    operation: Some(operation.to_owned()),
                    kind: None,
                    value: None,
                },
            },
            StorageError::AsyncRuntime { operation, message } => Self {
                status: DharaStatus::Error,
                payload: ErrorPayload {
                    code: "async_runtime",
                    message,
                    path: None,
                    operation: Some(operation.to_owned()),
                    kind: None,
                    value: None,
                },
            },
            StorageError::Watch { operation, message } => Self {
                status: DharaStatus::Error,
                payload: ErrorPayload {
                    code: "watch",
                    message,
                    path: None,
                    operation: Some(operation.to_owned()),
                    kind: None,
                    value: None,
                },
            },
        }
    }
}
