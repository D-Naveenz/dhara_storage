use anyhow::{Result, bail};

use crate::command::{ArgBinding, CommandSpec, FieldKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormValue {
    Text(String),
    Boolean(bool),
    Select(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandForm {
    pub command_id: &'static str,
    pub selected_field: usize,
    pub values: Vec<FormValue>,
}

impl CommandForm {
    pub fn from_command(command: &CommandSpec) -> Self {
        let values = command
            .ui
            .fields
            .iter()
            .map(|field| match field.kind {
                FieldKind::Text | FieldKind::Path => {
                    FormValue::Text(field.default_value.unwrap_or_default().to_owned())
                }
                FieldKind::Boolean => FormValue::Boolean(field.default_value == Some("true")),
                FieldKind::Select(options) => {
                    let default = field
                        .default_value
                        .unwrap_or(options.first().copied().unwrap_or(""));
                    let index = options
                        .iter()
                        .position(|option| *option == default)
                        .unwrap_or(0);
                    FormValue::Select(index)
                }
            })
            .collect();

        Self {
            command_id: command.id,
            selected_field: 0,
            values,
        }
    }

    pub fn selected_value(&self) -> Option<&FormValue> {
        self.values.get(self.selected_field)
    }

    pub fn selected_value_mut(&mut self) -> Option<&mut FormValue> {
        self.values.get_mut(self.selected_field)
    }

    pub fn move_next(&mut self, field_count: usize) {
        if field_count == 0 {
            self.selected_field = 0;
            return;
        }
        self.selected_field = (self.selected_field + 1) % field_count;
    }

    pub fn move_previous(&mut self, field_count: usize) {
        if field_count == 0 {
            self.selected_field = 0;
            return;
        }
        self.selected_field = (self.selected_field + field_count - 1) % field_count;
    }

    pub fn insert_char(&mut self, ch: char) {
        if let Some(FormValue::Text(value)) = self.selected_value_mut() {
            value.push(ch);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(FormValue::Text(value)) = self.selected_value_mut() {
            value.pop();
        }
    }

    pub fn toggle_bool(&mut self) {
        if let Some(FormValue::Boolean(value)) = self.selected_value_mut() {
            *value = !*value;
        }
    }

    pub fn cycle_next_option(&mut self, command: &CommandSpec) {
        let selected_field = self.selected_field;
        if let (Some(FormValue::Select(index)), Some(field)) = (
            self.selected_value_mut(),
            command.ui.fields.get(selected_field),
        ) {
            let FieldKind::Select(options) = field.kind else {
                return;
            };
            if options.is_empty() {
                *index = 0;
            } else {
                *index = (*index + 1) % options.len();
            }
        }
    }

    pub fn cycle_previous_option(&mut self, command: &CommandSpec) {
        let selected_field = self.selected_field;
        if let (Some(FormValue::Select(index)), Some(field)) = (
            self.selected_value_mut(),
            command.ui.fields.get(selected_field),
        ) {
            let FieldKind::Select(options) = field.kind else {
                return;
            };
            if options.is_empty() {
                *index = 0;
            } else {
                *index = (*index + options.len() - 1) % options.len();
            }
        }
    }

    pub fn display_value(&self, command: &CommandSpec, index: usize) -> String {
        match (
            self.values.get(index),
            command.ui.fields.get(index).map(|field| &field.kind),
        ) {
            (Some(FormValue::Text(value)), _) => value.clone(),
            (Some(FormValue::Boolean(value)), _) => {
                if *value {
                    "yes".to_owned()
                } else {
                    "no".to_owned()
                }
            }
            (Some(FormValue::Select(selected)), Some(FieldKind::Select(options))) => options
                .get(*selected)
                .copied()
                .unwrap_or_default()
                .to_owned(),
            _ => String::new(),
        }
    }

    pub fn build_args(&self, command: &CommandSpec) -> Result<Vec<String>> {
        let mut args = Vec::new();
        for (field, value) in command.ui.fields.iter().zip(self.values.iter()) {
            match (&field.binding, value) {
                (ArgBinding::Positional, FormValue::Text(text)) => {
                    let trimmed = text.trim();
                    if field.required && trimmed.is_empty() {
                        bail!("{} is required", field.label);
                    }
                    if !trimmed.is_empty() {
                        args.push(trimmed.to_owned());
                    }
                }
                (ArgBinding::FlagValue(flag), FormValue::Text(text)) => {
                    let trimmed = text.trim();
                    if field.required && trimmed.is_empty() {
                        bail!("{} is required", field.label);
                    }
                    if !trimmed.is_empty() {
                        args.push((*flag).to_owned());
                        args.push(trimmed.to_owned());
                    }
                }
                (ArgBinding::FlagValue("__mode"), FormValue::Select(index)) => {
                    let FieldKind::Select(options) = field.kind else {
                        continue;
                    };
                    if let Some(option) = options.get(*index) {
                        args.push(format!("--{option}"));
                    }
                }
                (ArgBinding::FlagValue(flag), FormValue::Select(index)) => {
                    let FieldKind::Select(options) = field.kind else {
                        continue;
                    };
                    if let Some(option) = options.get(*index) {
                        args.push((*flag).to_owned());
                        args.push((*option).to_owned());
                    }
                }
                (ArgBinding::Switch(flag), FormValue::Boolean(enabled)) if *enabled => {
                    args.push((*flag).to_owned());
                }
                _ => {}
            }
        }

        Ok(args)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;

    use crate::command::{
        ArgBinding, CommandResult, CommandSpec, CommandUi, FieldKind, FieldSpec, ToolContext,
    };

    use super::CommandForm;

    fn noop(_: &ToolContext, _: &[String]) -> Result<CommandResult> {
        Ok(CommandResult::success())
    }

    fn command(fields: Vec<FieldSpec>) -> CommandSpec {
        CommandSpec {
            id: "verify.package",
            path: &["verify", "package"],
            summary: "Verify package",
            args_summary: "",
            section: "verify",
            ui: CommandUi {
                description: "Verify package",
                fields,
                quick_run: true,
                supports_cancel: true,
            },
            handler: Arc::new(noop),
        }
    }

    #[test]
    fn build_args_serializes_flag_and_positional_fields() {
        let command = command(vec![
            FieldSpec {
                key: "version",
                label: "Version",
                help: "",
                kind: FieldKind::Text,
                binding: ArgBinding::Positional,
                required: true,
                default_value: Some("0.4.0"),
            },
            FieldSpec {
                key: "configuration",
                label: "Configuration",
                help: "",
                kind: FieldKind::Select(&["Release"]),
                binding: ArgBinding::FlagValue("--configuration"),
                required: true,
                default_value: Some("Release"),
            },
            FieldSpec {
                key: "check",
                label: "Check",
                help: "",
                kind: FieldKind::Boolean,
                binding: ArgBinding::Switch("--check"),
                required: false,
                default_value: Some("true"),
            },
        ]);

        let form = CommandForm::from_command(&command);
        let args = form.build_args(&command).unwrap();
        assert_eq!(
            args,
            vec![
                "0.4.0".to_owned(),
                "--configuration".to_owned(),
                "Release".to_owned(),
                "--check".to_owned()
            ]
        );
    }

    #[test]
    fn build_args_requires_missing_required_values() {
        let command = command(vec![FieldSpec {
            key: "version",
            label: "Version",
            help: "",
            kind: FieldKind::Text,
            binding: ArgBinding::Positional,
            required: true,
            default_value: None,
        }]);

        let form = CommandForm::from_command(&command);
        let error = form.build_args(&command).unwrap_err().to_string();
        assert!(error.contains("Version is required"));
    }
}
