use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Interactive,
    Direct,
}

impl RunMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::Direct => "direct",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolContext {
    pub repo_root: PathBuf,
    /// `exe_path` — directory containing the running executable; anchors logs, output, and artifacts.
    pub tool_root: PathBuf,
    pub run_mode: RunMode,
    pub min: bool,
    pub trace: bool,
    pub workers: usize,
    pub package_dir: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub logs_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportField {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredReport {
    pub title: String,
    pub fields: Vec<ReportField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub exit_code: i32,
    pub report: Option<StructuredReport>,
    pub message: Option<String>,
}

impl CommandResult {
    pub fn success() -> Self {
        Self {
            exit_code: 0,
            report: None,
            message: None,
        }
    }

    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            exit_code: 0,
            report: None,
            message: Some(message.into()),
        }
    }

    pub fn with_report(report: StructuredReport) -> Self {
        Self {
            exit_code: 0,
            report: Some(report),
            message: None,
        }
    }

    pub fn from_exit_code(exit_code: i32) -> Self {
        Self {
            exit_code,
            report: None,
            message: None,
        }
    }

    pub fn print(&self, context: &ToolContext) {
        if context.run_mode != RunMode::Direct {
            return;
        }

        if let Some(message) = &self.message {
            println!("{message}");
        }

        if let Some(report) = &self.report {
            println!("{}", report.title);
            for field in &report.fields {
                println!("{:<20} {}", field.label, field.value);
            }
        }
    }
}
