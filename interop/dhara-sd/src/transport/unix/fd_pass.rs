//! SCM_RIGHTS FD passing over a dedicated Unix domain socket.

use std::io::IoSlice;
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::Arc;

use nix::sys::socket::{ControlMessage, MsgFlags, UnixAddr, sendmsg};
use tonic::Status;

use crate::service::DaemonState;

/// Bind `path` and accept the host FD-pass connection in a background thread.
pub fn spawn_acceptor(
    path: &Path,
    state: Arc<DaemonState>,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(path)?;
    std::thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept()
            && let Ok(mut guard) = state.fd_pass_conn.lock()
        {
            *guard = Some(stream);
        }
    });
    Ok(())
}

/// Send one open file descriptor to the connected host.
pub fn send_fd(state: &DaemonState, fd: RawFd) -> Result<(), Status> {
    let mut guard = state
        .fd_pass_conn
        .lock()
        .map_err(|_| Status::internal("fd-pass connection lock poisoned"))?;
    let stream = guard
        .as_mut()
        .ok_or_else(|| Status::failed_precondition("fd-pass socket not connected"))?;

    let payload = [1u8];
    let iov = &[IoSlice::new(&payload)];
    let fds = [fd];
    // Connected socket: no destination address; pin UnixAddr so nix can infer `S`.
    sendmsg::<UnixAddr>(
        stream.as_raw_fd(),
        iov,
        &[ControlMessage::ScmRights(&fds)],
        MsgFlags::empty(),
        None,
    )
    .map_err(|err| Status::internal(format!("sendmsg SCM_RIGHTS failed: {err}")))?;
    Ok(())
}
