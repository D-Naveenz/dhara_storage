//! Windows named-pipe gRPC pilot daemon for Dhara Storage binding benchmarks.
//!
//! This binary is **not** a production binding. It exists so the C# harness can
//! compare in-process FFI (B1) against out-of-process gRPC (B2) on Windows.

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
    eprintln!("dhara-storage-daemon-pilot is Windows-only in this milestone.");
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
    use crate::service::{PilotState, create_service};

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let pipe_name = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_PIPE_NAME.to_string());

    info!(
        pipe = %pipe_name,
        pid = process::id(),
        "starting dhara-storage-daemon-pilot"
    );

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    runtime.block_on(async move {
        let state = Arc::new(PilotState::new(pipe_name.clone()));
        let service = create_service(state);

        if let Err(err) = serve_named_pipe(&pipe_name, service).await {
            error!(error = %err, "daemon terminated with error");
            process::exit(1);
        }
    });
}
