//! Exit the daemon when the binding host process disappears.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use tokio::time;
use tracing::info;

use crate::service::DaemonState;

/// On Linux, request SIGTERM when the spawning parent exits (kernel-level, no polling).
pub fn install_parent_death_signal() {
    #[cfg(target_os = "linux")]
    {
        use nix::sys::prctl::set_pdeathsig;
        use nix::sys::signal::Signal;

        if let Err(err) = set_pdeathsig(Signal::SIGTERM) {
            tracing::warn!(error = %err, "PR_SET_PDEATHSIG failed; parent watchdog remains active");
        }
    }
}

/// Poll interval for parent-process liveness.
const PARENT_POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Exit when no client completes handshake within this window.
const HANDSHAKE_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Spawn background tasks that exit the process when the host is gone or never connects.
pub fn spawn_lifecycle_tasks(state: Arc<DaemonState>) {
    spawn_handshake_idle_watch(state.clone());
}

/// Begin watching the handshake parent PID (idempotent).
pub fn spawn_parent_watchdog(state: Arc<DaemonState>, parent_pid: u32) {
    if state
        .parent_watch_started()
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    info!(parent_pid, "parent watchdog started");
    tokio::spawn(async move {
        let mut interval = time::interval(PARENT_POLL_INTERVAL);
        loop {
            interval.tick().await;
            let pid = state.parent_pid().unwrap_or(parent_pid);
            if !is_process_alive(pid) {
                info!(parent_pid = pid, "parent process exited; shutting down dhara-sd");
                std::process::exit(0);
            }
        }
    });
}

fn spawn_handshake_idle_watch(state: Arc<DaemonState>) {
    tokio::spawn(async move {
        time::sleep(HANDSHAKE_IDLE_TIMEOUT).await;
        if state.parent_pid().is_none() {
            info!("handshake not received within idle timeout; shutting down dhara-sd");
            std::process::exit(0);
        }
    });
}

fn is_process_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        is_process_alive_windows(pid)
    }
    #[cfg(unix)]
    {
        is_process_alive_unix(pid)
    }
}

#[cfg(windows)]
fn is_process_alive_windows(pid: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };

        let mut exit_code = 0u32;
        let alive = GetExitCodeProcess(handle, &mut exit_code).is_ok()
            && exit_code == STILL_ACTIVE.0 as u32;
        let _ = CloseHandle(handle);
        alive
    }
}

#[cfg(unix)]
fn is_process_alive_unix(pid: u32) -> bool {
    use nix::errno::Errno;
    use nix::sys::signal::kill;
    use nix::unistd::Pid;

    match kill(Pid::from_raw(pid as i32), None) {
        Ok(()) => true,
        Err(Errno::ESRCH) => false,
        Err(Errno::EPERM) => true,
        Err(_) => true,
    }
}
