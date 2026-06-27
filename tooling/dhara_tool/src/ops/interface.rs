use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolContext {
    pub repo_root: PathBuf,
    pub silent: bool,
    pub verbose: u8,
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
}

pub type CommandHandler = fn(&ToolContext, &[String]) -> anyhow::Result<CommandResult>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionSpec {
    pub name: &'static str,
    pub prompt: &'static str,
    pub summary: &'static str,
}

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub id: &'static str,
    pub path: &'static [&'static str],
    pub summary: &'static str,
    pub args_summary: &'static str,
    pub section: &'static str,
    pub handler: CommandHandler,
}
