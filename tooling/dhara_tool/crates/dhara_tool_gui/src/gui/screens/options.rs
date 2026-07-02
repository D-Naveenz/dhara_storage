use iced::widget::{checkbox, column, text};
use iced::{Element, Length};

use dhara_tool_cli::command::{CommandSpec, FieldKind};
use dhara_tool_cli::forms::{CommandForm, FormValue};

use super::super::app::Message;
use super::super::widgets::{
    field::labeled_field,
    input::text_field,
    path_field::browsable_path_field,
    select::dropdown,
};

pub fn view_options<'a>(
    command: &'a CommandSpec,
    form: &'a CommandForm,
) -> Element<'a, Message> {
    let mut fields = column![].spacing(8).width(Length::Fill);

    if !command.ui.description.is_empty() {
        fields = fields.push(text(command.ui.description).size(14));
    }

    for (index, field) in command.ui.fields.iter().enumerate() {
        let help = if field.help.is_empty() {
            None
        } else {
            Some(field.help)
        };

        let control: Element<'a, Message> = match (&field.kind, form.values.get(index)) {
            (FieldKind::Text | FieldKind::Path, Some(FormValue::Text(value))) => text_field(
                value,
                "",
                move |input| Message::FormTextChanged {
                    command_id: command.id,
                    field_index: index,
                    value: input,
                },
            ),
            (FieldKind::BrowsablePath { .. }, Some(FormValue::Text(value))) => {
                browsable_path_field(
                    value,
                    "",
                    move |input| Message::FormTextChanged {
                        command_id: command.id,
                        field_index: index,
                        value: input,
                    },
                    Message::FormBrowsePressed {
                        command_id: command.id,
                        field_index: index,
                    },
                )
            }
            (FieldKind::Boolean, Some(FormValue::Boolean(value))) => checkbox(*value)
                .label(field.label)
                .on_toggle(move |checked| Message::FormBooleanChanged {
                    command_id: command.id,
                    field_index: index,
                    value: checked,
                })
                .into(),
            (FieldKind::Select(options), Some(FormValue::Select(selected))) => {
                let current = options.get(*selected).copied();
                let options_owned: Vec<String> =
                    options.iter().map(|option| (*option).to_owned()).collect();
                dropdown(
                    options_owned,
                    current.map(str::to_owned),
                    "Select...",
                    move |choice| Message::FormSelectChanged {
                        command_id: command.id,
                        field_index: index,
                        value: choice,
                    },
                )
            }
            _ => text("").into(),
        };

        if matches!(field.kind, FieldKind::Boolean) {
            fields = fields.push(control);
        } else {
            fields = fields.push(labeled_field(field.label, help, control));
        }
    }

    if command.ui.fields.is_empty() {
        fields = fields.push(text("No options for this command.").size(14));
    }

    fields.into()
}
