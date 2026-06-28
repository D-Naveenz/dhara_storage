use std::io;

use rayon::ThreadPoolBuilder;

pub const DEFAULT_MAX_WORKERS: usize = 4;

/// Resolves the configured worker cap from CLI override or `TOOL_MAX_WORKERS` (default 4).
/// Ignores `RAYON_NUM_THREADS`.
pub fn resolve_max_workers(cli_override: Option<usize>) -> usize {
    if let Some(workers) = cli_override {
        return workers.max(1);
    }

    if let Ok(value) = std::env::var("TOOL_MAX_WORKERS")
        && let Ok(workers) = value.parse::<usize>()
    {
        return workers.max(1);
    }

    DEFAULT_MAX_WORKERS
}

/// Caps configured workers by available logical CPUs minus one (minimum 1).
pub fn effective_pool_threads(max_workers: usize) -> usize {
    let available = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(DEFAULT_MAX_WORKERS);
    let leave_one_free = if available > 1 { available - 1 } else { 1 };
    leave_one_free.min(max_workers).max(1)
}

/// Builds the global Rayon pool once per process. Returns the effective thread count.
pub fn init_global_thread_pool(cli_override: Option<usize>) -> io::Result<usize> {
    let max_workers = resolve_max_workers(cli_override);
    let threads = effective_pool_threads(max_workers);

    ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .map_err(io::Error::other)?;

    Ok(threads)
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_MAX_WORKERS, effective_pool_threads, resolve_max_workers};

    #[test]
    fn resolve_max_workers_defaults_to_four() {
        assert_eq!(resolve_max_workers(None), DEFAULT_MAX_WORKERS);
    }

    #[test]
    fn resolve_max_workers_cli_overrides_default() {
        assert_eq!(resolve_max_workers(Some(2)), 2);
    }

    #[test]
    fn effective_pool_threads_never_exceeds_cap() {
        assert!(effective_pool_threads(1) >= 1);
        assert!(effective_pool_threads(2) <= 2);
    }
}
