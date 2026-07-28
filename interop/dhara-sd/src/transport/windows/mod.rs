//! Windows named-pipe gRPC transport.

pub mod handle_dup;
pub mod pipe;

use std::sync::Arc;

use crate::service::{create_service, DaemonState};

/// Serve the daemon over a Windows named pipe.
pub async fn run(
    pipe_name: &str,
    state: Arc<DaemonState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let router = create_service(state);
    pipe::serve_named_pipe(pipe_name, router).await
}
