//! gRPC service implementation backed by `dhara_storage`.

use std::io::Cursor;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dhara_storage::{
    ContentKind, DirectoryDeleteOptions, DirectoryInfo, DirectoryStorage, FileInfo, SearchScope,
    SharedProgressReporter, StorageChangeType, StorageEntry, StorageProgress, StorageWatchConfig,
    TransferOptions, WriteOptions, analyze_path, copy_directory_with_options,
    copy_file_with_options, create_directory, create_directory_all, delete_directory_with_options,
    delete_file, move_directory_with_options, move_file_with_options, read_file, rename_directory,
    rename_file, write_file_from_reader,
};
use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::handle_dup;
use crate::pipe::DEFAULT_PIPE_NAME;

pub mod proto {
    tonic::include_proto!("dhara.sd.v1");
}

use proto::dhara_sd_server::{DharaSd, DharaSdServer};
use proto::*;

/// Shared daemon state for handshake PID and synthetic queue stubs.
pub struct DaemonState {
    pipe_name: String,
    parent_pid: AtomicU32,
    jobs: Mutex<Vec<String>>,
}

impl DaemonState {
    /// Create state for the given pipe path.
    pub fn new(pipe_name: String) -> Self {
        Self {
            pipe_name,
            parent_pid: AtomicU32::new(0),
            jobs: Mutex::new(Vec::new()),
        }
    }

    fn parent_pid(&self) -> Option<u32> {
        let pid = self.parent_pid.load(Ordering::SeqCst);
        if pid == 0 { None } else { Some(pid) }
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

        Ok(Response::new(HandshakeResponse {
            protocol_version: 1,
            pipe_name: self.state.pipe_name.clone(),
        }))
    }

    async fn get_file_info(
        &self,
        request: Request<GetFileInfoRequest>,
    ) -> Result<Response<GetFileInfoResponse>, Status> {
        let path = request.into_inner().path;
        let info = FileInfo::from_path(&path).map_err(map_storage_error)?;
        Ok(Response::new(GetFileInfoResponse {
            path: info.path().display().to_string(),
            name: info.name().to_string(),
            size: info.size(),
            extension: info.filename_extension().map(str::to_string),
            is_read_only: info.metadata().is_read_only(),
        }))
    }

    async fn get_directory_info(
        &self,
        request: Request<GetDirectoryInfoRequest>,
    ) -> Result<Response<GetDirectoryInfoResponse>, Status> {
        let path = request.into_inner().path;
        let exists = std::path::Path::new(&path).is_dir();
        if !exists {
            return Ok(Response::new(GetDirectoryInfoResponse {
                path: path.clone(),
                name: std::path::Path::new(&path)
                    .file_name()
                    .map(|v| v.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                exists: false,
            }));
        }
        let info = DirectoryInfo::from_path(&path).map_err(map_storage_error)?;
        Ok(Response::new(GetDirectoryInfoResponse {
            path: info.path().display().to_string(),
            name: info.name().to_string(),
            exists: true,
        }))
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
        let parent_pid = self.state.parent_pid().ok_or_else(|| {
            Status::failed_precondition("handshake required before OpenReadHandle")
        })?;
        let path = PathBuf::from(request.into_inner().path);
        let (handle, size) = handle_dup::open_read_and_duplicate(&path, parent_pid)
            .map_err(|err| Status::internal(err))?;
        Ok(Response::new(OpenReadHandleResponse { handle, size }))
    }

    async fn open_write_handle(
        &self,
        request: Request<OpenWriteHandleRequest>,
    ) -> Result<Response<OpenWriteHandleResponse>, Status> {
        let parent_pid = self.state.parent_pid().ok_or_else(|| {
            Status::failed_precondition("handshake required before OpenWriteHandle")
        })?;
        let req = request.into_inner();
        let path = PathBuf::from(req.path);
        let handle = handle_dup::open_write_and_duplicate(
            &path,
            parent_pid,
            req.overwrite,
            req.create_parent_directories,
        )
        .map_err(|err| Status::internal(err))?;
        Ok(Response::new(OpenWriteHandleResponse { handle }))
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
        if job_ids.is_empty() {
            if let Ok(jobs) = self.state.jobs.lock() {
                job_ids = jobs.clone();
            }
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

/// Expose the default pipe name for callers that only need the constant.
#[allow(dead_code)]
pub fn default_pipe_name() -> &'static str {
    DEFAULT_PIPE_NAME
}
