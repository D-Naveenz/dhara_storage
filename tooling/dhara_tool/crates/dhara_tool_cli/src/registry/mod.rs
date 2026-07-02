mod config;
mod defs;
mod package;
mod quality;
mod release;
mod ui;

use std::sync::Arc;

use anyhow::Result;

use crate::command::{
    CommandRegistry, CommandResult, CommandSpec, SectionSpec, ToolCapability, ToolContext,
};

pub(crate) struct RegisteredCommand {
    id: &'static str,
    path: &'static [&'static str],
    summary: &'static str,
    args_summary: &'static str,
    section: &'static str,
    handler: fn(&ToolContext, &[String]) -> Result<CommandResult>,
}

pub struct DharaStorageCapability;

impl ToolCapability for DharaStorageCapability {
    fn register(&self, registry: &mut CommandRegistry) {
        for section in self.sections() {
            registry.add_section(section);
        }

        for command in self.commands() {
            let handler = command.handler;
            registry.add_command(CommandSpec {
                id: command.id,
                path: command.path,
                summary: command.summary,
                args_summary: command.args_summary,
                section: command.section,
                ui: ui::ui_for_command(command.id, command.summary, command.args_summary),
                handler: Arc::new(handler),
            });
        }
    }
}

impl DharaStorageCapability {
    fn sections(&self) -> Vec<SectionSpec> {
        vec![
            config::section(),
            config::version_section(),
            defs::section(),
            quality::section(),
            package::native_section(),
            package::verify_section(),
            package::section(),
            release::section(),
        ]
    }

    fn commands(&self) -> Vec<RegisteredCommand> {
        let mut commands = Vec::new();
        commands.extend(config::commands());
        commands.extend(defs::commands());
        commands.extend(quality::commands());
        commands.extend(package::commands());
        commands.extend(release::commands());
        commands
    }
}

pub(crate) fn command(
    id: &'static str,
    path: &'static [&'static str],
    summary: &'static str,
    args_summary: &'static str,
    section: &'static str,
    handler: fn(&ToolContext, &[String]) -> Result<CommandResult>,
) -> RegisteredCommand {
    RegisteredCommand {
        id,
        path,
        summary,
        args_summary,
        section,
        handler,
    }
}

#[cfg(test)]
mod tests {
    use crate::command::{CommandRegistry, ToolCapability};

    use super::DharaStorageCapability;

    #[test]
    fn registration_adds_expected_sections_and_commands() {
        let mut registry = CommandRegistry::new();
        DharaStorageCapability.register(&mut registry);

        let sections = registry
            .sections()
            .map(|section| section.name)
            .collect::<Vec<_>>();
        assert_eq!(
            sections,
            vec![
                "config", "defs", "native", "package", "quality", "release", "verify", "version"
            ]
        );

        let commands = registry
            .commands()
            .map(|command| command.id)
            .collect::<Vec<_>>();
        assert!(commands.contains(&"config.show"));
        assert!(commands.contains(&"defs.inspect-trid-xml"));
        assert!(commands.contains(&"verify.package"));
        assert!(commands.contains(&"release.run"));
        assert!(
            registry
                .commands()
                .all(|command| !command.ui.description.trim().is_empty())
        );
    }
}
