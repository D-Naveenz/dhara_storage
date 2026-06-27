use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};

use crate::ops::{OutputCaptureGuard, OutputEvent, cancel_active_subprocess};

use crate::command::{CommandRegistry, CommandResult, RunMode, ToolContext};

pub struct RunHandle {
    pub label: String,
    pub cancelable: bool,
    pub output_rx: Receiver<OutputEvent>,
    pub completion_rx: Receiver<RunCompletion>,
    join: Option<JoinHandle<()>>,
}

#[derive(Debug)]
pub enum RunCompletion {
    Succeeded(CommandResult),
    Failed(String),
}

pub fn start_run(
    registry: CommandRegistry,
    context: ToolContext,
    command: Vec<String>,
    label: String,
    cancelable: bool,
) -> RunHandle {
    let (output_tx, output_rx) = mpsc::channel();
    let (completion_tx, completion_rx) = mpsc::channel();
    let mut context = context;
    context.run_mode = RunMode::Interactive;
    let join = thread::spawn(move || {
        let _capture = OutputCaptureGuard::install(output_tx);
        let completion = match registry.execute(&context, &command) {
            Ok(result) => RunCompletion::Succeeded(result),
            Err(error) => RunCompletion::Failed(format!("{error:#}")),
        };
        let _ = completion_tx.send(completion);
    });

    RunHandle {
        label,
        cancelable,
        output_rx,
        completion_rx,
        join: Some(join),
    }
}

impl RunHandle {
    pub fn try_take_completion(&mut self) -> Option<RunCompletion> {
        match self.completion_rx.try_recv() {
            Ok(completion) => {
                if let Some(join) = self.join.take() {
                    let _ = join.join();
                }
                Some(completion)
            }
            Err(_) => None,
        }
    }
}

pub fn cancel_run() -> bool {
    cancel_active_subprocess()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;

    use crate::command::{
        CommandRegistry, CommandResult, CommandSpec, CommandUi, RunMode, SectionSpec, ToolContext,
    };

    use super::{RunCompletion, start_run};

    fn report_handler(_: &ToolContext, _: &[String]) -> Result<CommandResult> {
        Ok(CommandResult::with_message("done"))
    }

    #[test]
    fn start_run_returns_completion() {
        let mut registry = CommandRegistry::new();
        registry.add_section(SectionSpec {
            name: "config",
            prompt: "cfg> ",
            summary: "Config",
        });
        registry.add_command(CommandSpec {
            id: "config.show",
            path: &["config", "show"],
            summary: "Show",
            args_summary: "",
            section: "config",
            ui: CommandUi::empty("Show"),
            handler: Arc::new(report_handler),
        });

        let context = ToolContext {
            repo_root: ".".into(),
            run_mode: RunMode::Interactive,
            verbose: 0,
            quiet: false,
            package_dir: None,
            output_dir: None,
            logs_dir: None,
        };

        let mut run = start_run(
            registry,
            context,
            vec!["config".to_owned(), "show".to_owned()],
            "config show".to_owned(),
            false,
        );

        let completion = loop {
            if let Some(completion) = run.try_take_completion() {
                break completion;
            }
        };

        match completion {
            RunCompletion::Succeeded(result) => {
                assert_eq!(result.message.as_deref(), Some("done"));
            }
            RunCompletion::Failed(error) => panic!("unexpected failure: {error}"),
        }
    }
}
