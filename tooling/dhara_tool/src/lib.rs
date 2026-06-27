pub mod app;
pub mod capabilities;
pub mod command;
pub mod ops;
pub mod package;
pub mod process;
pub mod tui;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
