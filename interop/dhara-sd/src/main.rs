//! Dhara Storage daemon (`dhara-sd`) — gRPC control plane for foreign-language bindings.
//!
//! Data plane uses OS handle duplication (Windows) or FD passing (Unix). Do not ship large
//! payloads over gRPC as the product path.

#![deny(missing_docs)]

mod log_broadcast;
mod parent_watch;
mod proto;
mod service;
mod transport;

use std::env;
use std::process;
use std::sync::Arc;

use tracing::error;
use tracing::info;

use crate::log_broadcast::init_tracing;
use crate::service::DaemonState;

fn main() {
    parent_watch::install_parent_death_signal();
    let log_tx = init_tracing();

    let endpoint = env::args().nth(1).unwrap_or_else(default_endpoint);

    info!(endpoint = %endpoint, pid = process::id(), "starting dhara-sd");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    runtime.block_on(async move {
        let state = Arc::new(DaemonState::new(endpoint.clone(), log_tx));
        parent_watch::spawn_lifecycle_tasks(state.clone());

        let result = run_transport(&endpoint, state).await;
        if let Err(err) = result {
            error!(error = %err, "daemon terminated with error");
            process::exit(1);
        }
    });
}

async fn run_transport(
    endpoint: &str,
    state: Arc<DaemonState>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        transport::windows::run(endpoint, state).await
    }
    #[cfg(unix)]
    {
        transport::unix::run(endpoint, state).await
    }
}

#[cfg(windows)]
fn default_endpoint() -> String {
    transport::windows::pipe::DEFAULT_PIPE_NAME.to_string()
}

#[cfg(unix)]
fn default_endpoint() -> String {
    let dir = env::temp_dir().join(format!("dhara-sd-{}", process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir.display().to_string()
}
