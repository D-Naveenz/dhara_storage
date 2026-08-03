//! Unix domain socket gRPC transport for tonic.

use std::path::Path;

use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::server::Router;

/// Serve the tonic router on a bound Unix domain socket.
pub async fn serve_grpc(
    socket_path: &Path,
    router: Router,
) -> Result<(), Box<dyn std::error::Error>> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(socket_path)?;
    let incoming = UnixListenerStream::new(listener);
    router.serve_with_incoming(incoming).await?;
    Ok(())
}
