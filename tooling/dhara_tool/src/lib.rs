pub mod app;
pub mod command;
pub mod ops;
pub mod paths;
pub mod process;
pub mod tui;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
