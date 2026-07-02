use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct GuiBootParams {
    pub min: bool,
    pub trace: bool,
    pub workers: usize,
    pub yes: bool,
    pub package_dir: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub logs_dir: Option<PathBuf>,
}
