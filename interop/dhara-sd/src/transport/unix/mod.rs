//! Linux/macOS UDS gRPC + SCM_RIGHTS data plane.

pub mod fd_pass;
pub mod uds;

use std::path::PathBuf;
use std::sync::Arc;

use crate::service::{DaemonState, create_service};

/// Serve the daemon over Unix domain sockets.
pub async fn run(
    endpoint_dir: &str,
    state: Arc<DaemonState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(endpoint_dir);
    std::fs::create_dir_all(&dir)?;

    let grpc_path = dir.join("grpc.sock");
    let fd_path = dir.join("fd.sock");

    state.set_control_endpoint(grpc_path.display().to_string());
    state.set_fd_pass_endpoint(fd_path.display().to_string());

    fd_pass::spawn_acceptor(&fd_path, state.clone())?;

    let router = create_service(state);
    uds::serve_grpc(&grpc_path, router).await
}
