mod audit;
mod progress;

pub use audit::*;
pub use progress::{
    dispatch_trid_progress, emit_trid_progress, init_progress_settings, reset_build_progress_logging,
};

pub fn interactive_mode_enabled() -> bool {
    progress::progress_settings().run_mode == crate::context::RunMode::Interactive
}
