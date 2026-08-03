//! gRPC service implementation backed by `dhara_storage`.

use std::io::Cursor;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
#[cfg(unix)]
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dhara_storage::{
    ContentKind, DEFAULT_SHELL_ICON_SIZE, DirectoryDeleteOptions, DirectoryInfo, DirectoryStorage,
    FileInfo, SearchScope, SharedProgressReporter, ShellIcon, StorageChangeType, StorageEntry,
    StorageProgress, StorageWatchConfig, TransferOptions, WriteOptions, analyze_path,
    copy_directory_with_options, copy_file_with_options, create_directory, create_directory_all,
    delete_directory_with_options, delete_file, move_directory_with_options,
    move_file_with_options, read_file, rename_directory, rename_file, write_file_from_reader,
};
use futures::{Stream, StreamExt};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio_stream::wrappers::{BroadcastStream, ReceiverStream};
use tonic::{Request, Response, Status};
use tracing::{debug, warn};
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(unix)]
use std::os::unix::net::UnixStream;

use crate::log_broadcast::level_rank;
use crate::proto::dhara_sd_server::{DharaSd, DharaSdServer};
use crate::proto::*;
#[cfg(unix)]
use crate::transport::unix::fd_pass;
#[cfg(windows)]
use crate::transport::windows::handle_dup;

/// Shared daemon state for handshake PID, logging, and synthetic queue stubs.
pub struct DaemonState {
    control_endpoint: std::sync::RwLock<String>,
    #[cfg(unix)]
    fd_pass_endpoint: std::sync::RwLock<String>,
    parent_pid: AtomicU32,
    parent_watch_started: AtomicBool,
    jobs: std::sync::Mutex<Vec<String>>,
    log_tx: broadcast::Sender<LogRecord>,
    #[cfg(unix)]
    pub(crate) fd_pass_conn: Mutex<Option<UnixStream>>,
}

impl DaemonState {
    /// Create state for the given control endpoint path.
    pub fn new(control_endpoint: String, log_tx: broadcast::Sender<LogRecord>) -> Self {
        Self {
            control_endpoint: std::sync::RwLock::new(control_endpoint),
            #[cfg(unix)]
            fd_pass_endpoint: std::sync::RwLock::new(String::new()),
            parent_pid: AtomicU32::new(0),
            parent_watch_started: AtomicBool::new(false),
            jobs: std::sync::Mutex::new(Vec::new()),
            log_tx,
            #[cfg(unix)]
            fd_pass_conn: Mutex::new(None),
        }
    }

    /// Update the gRPC control endpoint advertised in `Handshake`.
    #[cfg(unix)]
    pub fn set_control_endpoint(&self, endpoint: String) {
        if let Ok(mut guard) = self.control_endpoint.write() {
            *guard = endpoint;
        }
    }

    /// Update the FD-pass endpoint advertised in `Handshake`.
    #[cfg(unix)]
    pub fn set_fd_pass_endpoint(&self, endpoint: String) {
        if let Ok(mut guard) = self.fd_pass_endpoint.write() {
            *guard = endpoint;
        }
    }

    fn control_endpoint(&self) -> String {
        self.control_endpoint
            .read()
            .map(|value| value.clone())
            .unwrap_or_default()
    }

    fn fd_pass_endpoint(&self) -> String {
        #[cfg(windows)]
        {
            String::new()
        }
        #[cfg(unix)]
        {
            self.fd_pass_endpoint
                .read()
                .map(|value| value.clone())
                .unwrap_or_default()
        }
    }

    pub(crate) fn parent_pid(&self) -> Option<u32> {
        let pid = self.parent_pid.load(Ordering::SeqCst);
        if pid == 0 { None } else { Some(pid) }
    }

    pub(crate) fn parent_watch_started(&self) -> &AtomicBool {
        &self.parent_watch_started
    }
}

/// Build the tonic service registered on the named-pipe server.
pub fn create_service(state: Arc<DaemonState>) -> tonic::transport::server::Router {
    tonic::transport::Server::builder().add_service(DharaSdServer::new(DharaSdService { state }))
}

struct DharaSdService {
    state: Arc<DaemonState>,
}

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

#[tonic::async_trait]
impl DharaSd for DharaSdService {
    async fn ping(&self, _request: Request<PingRequest>) -> Result<Response<PingResponse>, Status> {
        Ok(Response::new(PingResponse {
            unix_millis: unix_millis_now(),
        }))
    }

    async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
        Ok(Response::new(EchoResponse {
            payload: request.into_inner().payload,
        }))
    }

    async fn handshake(
        &self,
        request: Request<HandshakeRequest>,
    ) -> Result<Response<HandshakeResponse>, Status> {
        let req = request.into_inner();
        if req.protocol_version == 0 {
            return Err(Status::invalid_argument("protocol_version must be >= 1"));
        }
        if req.parent_pid == 0 {
            return Err(Status::invalid_argument("parent_pid must be non-zero"));
        }

        self.state
            .parent_pid
            .store(req.parent_pid, Ordering::SeqCst);
        debug!(parent_pid = req.parent_pid, "handshake accepted");
        crate::parent_watch::spawn_parent_watchdog(self.state.clone(), req.parent_pid);

        Ok(Response::new(HandshakeResponse {
            protocol_version: 1,
            pipe_name: self.state.control_endpoint(),
            fd_pass_endpoint: self.state.fd_pass_endpoint(),
        }))
    }

    async fn get_file_info(
        &self,
        request: Request<GetFileInfoRequest>,
    ) -> Result<Response<GetFileInfoResponse>, Status> {
        let req = request.into_inner();
        let path = req.path;
        let include_shell = req.include_shell_details;
        let include_icon = req.include_icon;
        let icon_size = if req.icon_size == 0 {
            DEFAULT_SHELL_ICON_SIZE
        } else {
            req.icon_size
        };

        let response = tokio::task::spawn_blocking(move || {
            let info = FileInfo::from_path(&path)?;
            let (shell_display_name, shell_type_name) = if include_shell {
                info.shell_details()
                    .map(|details| (details.display_name.clone(), details.type_name.clone()))
                    .unwrap_or((None, None))
            } else {
                (None, None)
            };
            let icon = if include_icon {
                info.load_icon_at(icon_size).map(shell_icon_to_proto)
            } else {
                None
            };
            Ok::<_, dhara_storage::StorageError>(GetFileInfoResponse {
                path: info.path().display().to_string(),
                name: info.name().to_string(),
                size: info.size(),
                extension: info.filename_extension().map(str::to_string),
                is_read_only: info.metadata().is_read_only(),
                shell_display_name,
                shell_type_name,
                icon,
            })
        })
        .await
        .map_err(|err| Status::internal(err.to_string()))?
        .map_err(map_storage_error)?;

        Ok(Response::new(response))
    }

    async fn get_directory_info(
        &self,
        request: Request<GetDirectoryInfoRequest>,
    ) -> Result<Response<GetDirectoryInfoResponse>, Status> {
        let req = request.into_inner();
        let path = req.path;
        let include_shell = req.include_shell_details;
        let include_icon = req.include_icon;
        let icon_size = if req.icon_size == 0 {
            DEFAULT_SHELL_ICON_SIZE
        } else {
            req.icon_size
        };

        let response = tokio::task::spawn_blocking(move || {
            let exists = std::path::Path::new(&path).is_dir();
            if !exists {
                return Ok(GetDirectoryInfoResponse {
                    path: path.clone(),
                    name: std::path::Path::new(&path)
                        .file_name()
                        .map(|value| value.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    exists: false,
                    shell_display_name: None,
                    shell_type_name: None,
                    icon: None,
                });
            }

            let info = DirectoryInfo::from_path(&path)?;
            let (shell_display_name, shell_type_name) = if include_shell {
                info.shell_details()
                    .map(|details| (details.display_name.clone(), details.type_name.clone()))
                    .unwrap_or((None, None))
            } else {
                (None, None)
            };
            let icon = if include_icon {
                info.load_icon_at(icon_size).map(shell_icon_to_proto)
            } else {
                None
            };

            Ok(GetDirectoryInfoResponse {
                path: info.path().display().to_string(),
                name: info.name().to_string(),
                exists: true,
                shell_display_name,
                shell_type_name,
                icon,
            })
        })
        .await
        .map_err(|err| Status::internal(err.to_string()))?
        .map_err(map_storage_error)?;

        Ok(Response::new(response))
    }

    async fn list_entries(
        &self,
        request: Request<ListEntriesRequest>,
    ) -> Result<Response<ListEntriesResponse>, Status> {
        let req = request.into_inner();
        let directory = DirectoryStorage::from_existing(&req.path).map_err(map_storage_error)?;
        let scope = if req.recursive {
            SearchScope::AllDirectories
        } else {
            SearchScope::TopDirectoryOnly
        };
        let entries = directory
            .entries_matching("*", scope)
            .map_err(map_storage_error)?
            .into_iter()
            .map(|entry| {
                let is_directory = matches!(entry, StorageEntry::Directory(_));
                let path = entry.path().display().to_string();
                let name = entry
                    .path()
                    .file_name()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_default();
                EntryInfo {
                    path,
                    name,
                    is_directory,
                }
            })
            .collect();

        Ok(Response::new(ListEntriesResponse { entries }))
    }

    async fn analyze_path(
        &self,
        request: Request<AnalyzePathRequest>,
    ) -> Result<Response<AnalyzePathResponse>, Status> {
        let path = request.into_inner().path;
        let report = tokio::task::spawn_blocking(move || analyze_path(&path))
            .await
            .map_err(|err| Status::internal(err.to_string()))?
            .map_err(map_storage_error)?;
        Ok(Response::new(AnalyzePathResponse {
            top_mime_type: report.top_mime_type,
            top_detected_extension: report.top_detected_extension,
            content_kind: content_kind_to_u32(report.content_kind),
            bytes_scanned: report.bytes_scanned as u64,
            file_size: report.file_size,
            matches: report
                .matches
                .into_iter()
                .map(|item| DetectedMatch {
                    file_type_label: item.file_type_label,
                    mime_type: item.mime_type,
                    confidence: item.confidence,
                    score: item.score,
                })
                .collect(),
        }))
    }

    async fn open_read_handle(
        &self,
        request: Request<OpenReadHandleRequest>,
    ) -> Result<Response<OpenReadHandleResponse>, Status> {
        self.state.parent_pid().ok_or_else(|| {
            Status::failed_precondition("handshake required before OpenReadHandle")
        })?;

        let path = PathBuf::from(request.into_inner().path);

        #[cfg(windows)]
        {
            let parent_pid = self.state.parent_pid().expect("checked above");
            let (handle, size) =
                handle_dup::open_read_and_duplicate(&path, parent_pid).map_err(Status::internal)?;
            return Ok(Response::new(OpenReadHandleResponse { handle, size }));
        }

        #[cfg(unix)]
        {
            let state = self.state.clone();
            let size = tokio::task::spawn_blocking(move || {
                let file = std::fs::File::open(&path).map_err(map_io_error)?;
                let size = file.metadata().map_err(map_io_error)?.len();
                fd_pass::send_fd(&state, file.as_raw_fd())?;
                Ok::<_, Status>(size)
            })
            .await
            .map_err(|err| Status::internal(err.to_string()))??;

            Ok(Response::new(OpenReadHandleResponse { handle: 0, size }))
        }
    }

    async fn open_write_handle(
        &self,
        request: Request<OpenWriteHandleRequest>,
    ) -> Result<Response<OpenWriteHandleResponse>, Status> {
        self.state.parent_pid().ok_or_else(|| {
            Status::failed_precondition("handshake required before OpenWriteHandle")
        })?;

        let req = request.into_inner();
        let path = PathBuf::from(req.path);

        #[cfg(windows)]
        {
            let parent_pid = self.state.parent_pid().expect("checked above");
            let handle = handle_dup::open_write_and_duplicate(
                &path,
                parent_pid,
                req.overwrite,
                req.create_parent_directories,
            )
            .map_err(Status::internal)?;
            return Ok(Response::new(OpenWriteHandleResponse { handle }));
        }

        #[cfg(unix)]
        {
            let state = self.state.clone();
            let overwrite = req.overwrite;
            let create_parents = req.create_parent_directories;
            tokio::task::spawn_blocking(move || {
                if create_parents && let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(map_io_error)?;
                }

                let file = if overwrite {
                    std::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&path)
                } else if path.exists() {
                    std::fs::OpenOptions::new().write(true).open(&path)
                } else {
                    std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path)
                }
                .map_err(map_io_error)?;

                let fd = file.as_raw_fd();
                fd_pass::send_fd(&state, fd)?;
                Ok::<_, Status>(())
            })
            .await
            .map_err(|err| Status::internal(err.to_string()))??;

            Ok(Response::new(OpenWriteHandleResponse { handle: 0 }))
        }
    }

    async fn create_directory(
        &self,
        request: Request<CreateDirectoryRequest>,
    ) -> Result<Response<CreateDirectoryResponse>, Status> {
        let req = request.into_inner();
        let path = tokio::task::spawn_blocking(move || {
            if req.create_parents {
                create_directory_all(&req.path)
            } else {
                create_directory(&req.path)
            }
        })
        .await
        .map_err(|err| Status::internal(err.to_string()))?
        .map_err(map_storage_error)?;
        Ok(Response::new(CreateDirectoryResponse {
            path: path.display().to_string(),
        }))
    }

    async fn delete_path(
        &self,
        request: Request<DeletePathRequest>,
    ) -> Result<Response<DeletePathResponse>, Status> {
        let req = request.into_inner();
        tokio::task::spawn_blocking(move || {
            let path = PathBuf::from(&req.path);
            if path.is_dir() {
                delete_directory_with_options(
                    &path,
                    DirectoryDeleteOptions {
                        recursive: req.recursive,
                        ..DirectoryDeleteOptions::default()
                    },
                )
            } else {
                delete_file(&path)
            }
        })
        .await
        .map_err(|err| Status::internal(err.to_string()))?
        .map_err(map_storage_error)?;
        Ok(Response::new(DeletePathResponse {}))
    }

    async fn move_path(
        &self,
        request: Request<MovePathRequest>,
    ) -> Result<Response<MovePathResponse>, Status> {
        let req = request.into_inner();
        let destination = tokio::task::spawn_blocking(move || {
            let source = PathBuf::from(&req.source);
            let options = TransferOptions {
                overwrite: req.overwrite,
                ..TransferOptions::default()
            };
            if source.is_dir() {
                move_directory_with_options(&source, &req.destination, options)
            } else {
                move_file_with_options(&source, &req.destination, options)
            }
        })
        .await
        .map_err(|err| Status::internal(err.to_string()))?
        .map_err(map_storage_error)?;
        Ok(Response::new(MovePathResponse {
            path: destination.display().to_string(),
        }))
    }

    async fn rename_path(
        &self,
        request: Request<RenamePathRequest>,
    ) -> Result<Response<RenamePathResponse>, Status> {
        let req = request.into_inner();
        let destination = tokio::task::spawn_blocking(move || {
            let source = PathBuf::from(&req.path);
            if source.is_dir() {
                rename_directory(&source, &req.new_name)
            } else {
                rename_file(&source, &req.new_name)
            }
        })
        .await
        .map_err(|err| Status::internal(err.to_string()))?
        .map_err(map_storage_error)?;
        Ok(Response::new(RenamePathResponse {
            path: destination.display().to_string(),
        }))
    }

    async fn read_file_bytes(
        &self,
        request: Request<ReadFileBytesRequest>,
    ) -> Result<Response<ReadFileBytesResponse>, Status> {
        let path = request.into_inner().path;
        let data = tokio::task::spawn_blocking(move || read_file(&path))
            .await
            .map_err(|err| Status::internal(err.to_string()))?
            .map_err(map_storage_error)?;
        Ok(Response::new(ReadFileBytesResponse { data }))
    }

    async fn write_file_bytes(
        &self,
        request: Request<WriteFileBytesRequest>,
    ) -> Result<Response<WriteFileBytesResponse>, Status> {
        let req = request.into_inner();
        let bytes_written = req.data.len() as u64;
        let written_path = tokio::task::spawn_blocking(move || {
            let options = WriteOptions {
                overwrite: req.overwrite,
                create_parent_directories: req.create_parent_directories,
                ..WriteOptions::default()
            };
            let mut cursor = Cursor::new(req.data);
            write_file_from_reader(&req.path, &mut cursor, options)
        })
        .await
        .map_err(|err| Status::internal(err.to_string()))?
        .map_err(map_storage_error)?;
        Ok(Response::new(WriteFileBytesResponse {
            path: written_path.display().to_string(),
            bytes_written,
        }))
    }

    type CopyFileStream = ResponseStream<CopyFileProgress>;

    async fn copy_file(
        &self,
        request: Request<CopyFileRequest>,
    ) -> Result<Response<Self::CopyFileStream>, Status> {
        let req = request.into_inner();
        Ok(Response::new(spawn_copy_progress(move |reporter| {
            let options = TransferOptions {
                overwrite: req.overwrite,
                progress: Some(reporter),
                ..TransferOptions::default()
            };
            copy_file_with_options(&req.source, &req.destination, options)
        })))
    }

    type CopyDirectoryStream = ResponseStream<CopyFileProgress>;

    async fn copy_directory(
        &self,
        request: Request<CopyDirectoryRequest>,
    ) -> Result<Response<Self::CopyDirectoryStream>, Status> {
        let req = request.into_inner();
        Ok(Response::new(spawn_copy_progress(move |reporter| {
            let options = TransferOptions {
                overwrite: req.overwrite,
                progress: Some(reporter),
                ..TransferOptions::default()
            };
            copy_directory_with_options(&req.source, &req.destination, options)
        })))
    }

    type WatchDirectoryStream = ResponseStream<WatchEvent>;

    async fn watch_directory(
        &self,
        request: Request<WatchDirectoryRequest>,
    ) -> Result<Response<Self::WatchDirectoryStream>, Status> {
        let req = request.into_inner();
        let path = PathBuf::from(req.path);
        let debounce = if req.debounce_window_ms == 0 {
            Duration::from_millis(50)
        } else {
            Duration::from_millis(u64::from(req.debounce_window_ms))
        };

        let handle = DirectoryStorage::from_existing(&path)
            .map_err(map_storage_error)?
            .watch(StorageWatchConfig {
                recursive: req.recursive,
                debounce_window: debounce,
            })
            .map_err(map_storage_error)?;

        let (tx, rx) = mpsc::channel::<Result<WatchEvent, Status>>(64);
        tokio::task::spawn_blocking(move || {
            loop {
                match handle.recv_timeout(Duration::from_millis(250)) {
                    Ok(Some(event)) => {
                        let payload = WatchEvent {
                            change_type: change_type_to_u32(event.change_type),
                            path: event.path.display().to_string(),
                            previous_path: event
                                .previous_path
                                .map(|value| value.display().to_string()),
                            observed_unix_millis: system_time_millis(event.observed_at),
                        };
                        if tx.blocking_send(Ok(payload)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        if tx.is_closed() {
                            break;
                        }
                    }
                    Err(err) => {
                        let _ = tx.blocking_send(Err(map_storage_error(err)));
                        break;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn enqueue_work(
        &self,
        request: Request<EnqueueWorkRequest>,
    ) -> Result<Response<EnqueueWorkResponse>, Status> {
        let req = request.into_inner();
        let job_id = format!("{}:{}", req.kind, Uuid::new_v4());
        if let Ok(mut jobs) = self.state.jobs.lock() {
            jobs.push(job_id.clone());
        }
        Ok(Response::new(EnqueueWorkResponse { job_id }))
    }

    type StreamWorkEventsStream = ResponseStream<WorkEvent>;

    async fn stream_work_events(
        &self,
        request: Request<StreamWorkEventsRequest>,
    ) -> Result<Response<Self::StreamWorkEventsStream>, Status> {
        let req = request.into_inner();
        let mut job_ids = req.job_ids;
        if job_ids.is_empty()
            && let Ok(jobs) = self.state.jobs.lock()
        {
            job_ids = jobs.clone();
        }

        let synthetic = if req.synthetic_count == 0 {
            job_ids.len() as u32
        } else {
            req.synthetic_count
        };

        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(async move {
            for index in 0..synthetic {
                let job_id = job_ids
                    .get(index as usize)
                    .cloned()
                    .unwrap_or_else(|| format!("synthetic-{index}"));
                if tx
                    .send(Ok(WorkEvent {
                        job_id,
                        state: "completed".into(),
                        unix_millis: unix_millis_now(),
                    }))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    type StreamLogsStream = ResponseStream<LogRecord>;

    async fn stream_logs(
        &self,
        request: Request<StreamLogsRequest>,
    ) -> Result<Response<Self::StreamLogsStream>, Status> {
        self.state
            .parent_pid()
            .ok_or_else(|| Status::failed_precondition("handshake required before StreamLogs"))?;

        let min_level = request.into_inner().min_level;
        let min_level = if min_level == 0 { 3 } else { min_level };
        let rx = self.state.log_tx.subscribe();
        let stream = BroadcastStream::new(rx).filter_map(move |result| {
            futures::future::ready(match result {
                Ok(record) if level_rank(&record.level) >= min_level => Some(Ok(record)),
                Ok(_) => None,
                Err(_) => None,
            })
        });

        Ok(Response::new(Box::pin(stream)))
    }
}

fn shell_icon_to_proto(icon: ShellIcon) -> ShellIconPayload {
    ShellIconPayload {
        width: icon.width,
        height: icon.height,
        rgba_pixels: icon.rgba,
    }
}

fn spawn_copy_progress<F>(work: F) -> ResponseStream<CopyFileProgress>
where
    F: FnOnce(SharedProgressReporter) -> Result<PathBuf, dhara_storage::StorageError>
        + Send
        + 'static,
{
    let (tx, rx) = mpsc::channel::<Result<CopyFileProgress, Status>>(256);
    tokio::task::spawn_blocking(move || {
        let progress_tx = tx.clone();
        let reporter: SharedProgressReporter = Arc::new(move |progress: StorageProgress| {
            let _ = progress_tx.blocking_send(Ok(CopyFileProgress {
                bytes_transferred: progress.bytes_transferred,
                total_bytes: progress.total_bytes,
                bytes_per_second: progress.bytes_per_second,
                completed: false,
                destination: None,
                error_message: None,
            }));
        });

        match work(reporter) {
            Ok(destination) => {
                let _ = tx.blocking_send(Ok(CopyFileProgress {
                    bytes_transferred: 0,
                    total_bytes: None,
                    bytes_per_second: 0.0,
                    completed: true,
                    destination: Some(destination.display().to_string()),
                    error_message: None,
                }));
            }
            Err(err) => {
                let _ = tx.blocking_send(Ok(CopyFileProgress {
                    bytes_transferred: 0,
                    total_bytes: None,
                    bytes_per_second: 0.0,
                    completed: true,
                    destination: None,
                    error_message: Some(err.to_string()),
                }));
            }
        }
    });
    Box::pin(ReceiverStream::new(rx))
}

fn map_storage_error(err: dhara_storage::StorageError) -> Status {
    warn!(error = %err, "storage operation failed");
    Status::internal(err.to_string())
}

#[cfg(unix)]
fn map_io_error(err: std::io::Error) -> Status {
    warn!(error = %err, "I/O operation failed");
    Status::internal(err.to_string())
}

fn content_kind_to_u32(kind: ContentKind) -> u32 {
    match kind {
        ContentKind::Text => 1,
        ContentKind::Binary => 2,
        ContentKind::Unknown => 0,
    }
}

fn change_type_to_u32(kind: StorageChangeType) -> u32 {
    match kind {
        StorageChangeType::Created => 1,
        StorageChangeType::Deleted => 2,
        StorageChangeType::Modified => 3,
        StorageChangeType::Relocated => 4,
    }
}

fn unix_millis_now() -> i64 {
    system_time_millis(SystemTime::now())
}

fn system_time_millis(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}
