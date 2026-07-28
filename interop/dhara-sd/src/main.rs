//! Dhara Storage daemon (`dhara-sd`) — gRPC control plane for foreign-language bindings.
//!
//! Data plane uses OS handle duplication (Windows) or FD passing (planned). Do not ship
//! large payloads over gRPC as the product path.

#![deny(missing_docs)]

#[cfg(windows)]
mod handle_dup;
#[cfg(windows)]
mod pipe;
#[cfg(windows)]
mod service;

#[cfg(windows)]
fn main() {
    windows_main();
}

#[cfg(not(windows))]
fn main() {
    // Cross-platform UDS + SCM_RIGHTS is documented; Windows named pipes ship first.
    eprintln!("dhara-sd: non-Windows transports are not implemented yet (see docs/daemon-transport.md).");
    std::process::exit(2);
}

#[cfg(windows)]
fn windows_main() {
    use std::env;
    use std::process;
    use std::sync::Arc;

    use tracing::{error, info};
    use tracing_subscriber::EnvFilter;

    use crate::pipe::{DEFAULT_PIPE_NAME, serve_named_pipe};
    use crate::service::{DaemonState, create_service};

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let pipe_name = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_PIPE_NAME.to_string());

    info!(pipe = %pipe_name, pid = process::id(), "starting dhara-sd");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    runtime.block_on(async move {
        let state = Arc::new(DaemonState::new(pipe_name.clone()));
        let service = create_service(state);

        if let Err(err) = serve_named_pipe(&pipe_name, service).await {
            error!(error = %err, "daemon terminated with error");
            process::exit(1);
        }
    });
}
