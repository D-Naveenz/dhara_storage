use iced::widget::{checkbox, column, pick_list, text, text_input};
use iced::{Element, Length};

use crate::command::{CommandSpec, FieldKind};
use crate::ui::{CommandForm, FormValue};

use super::app::Message;

pub fn view_options_form<'a>(
    command: &'a CommandSpec,
    form: &'a CommandForm,
) -> Element<'a, Message> {
    let mut fields = column![].spacing(8).width(Length::Fill);

    if !command.ui.description.is_empty() {
        fields = fields.push(text(command.ui.description).size(14));
    }

    for (index, field) in command.ui.fields.iter().enumerate() {
        let label = text(format!("{}:", field.label)).size(14);
        let help = if field.help.is_empty() {
            None
        } else {
            Some(text(field.help).size(12))
        };

        let control: Element<'a, Message> = match (&field.kind, form.values.get(index)) {
            (FieldKind::Text | FieldKind::Path, Some(FormValue::Text(value))) => text_input("", value)
                .on_input(move |input| Message::FormTextChanged {
                    command_id: command.id,
                    field_index: index,
                    value: input,
                })
                .padding(6)
                .width(Length::Fill)
                .into(),
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
                let options_owned: Vec<String> = options.iter().map(|option| (*option).to_owned()).collect();
                pick_list(options_owned, current.map(str::to_owned), move |choice| {
                    Message::FormSelectChanged {
                        command_id: command.id,
                        field_index: index,
                        value: choice,
                    }
                })
                .placeholder("Select...")
                .width(Length::Fill)
                .into()
            }
            _ => text("").into(),
        };

        let mut field_column = column![label].spacing(4);
        if let FieldKind::Boolean = field.kind {
            field_column = field_column.push(control);
        } else {
            field_column = field_column.push(control);
        }
        if let Some(help) = help {
            field_column = field_column.push(help);
        }
        fields = fields.push(field_column);
    }

    if command.ui.fields.is_empty() {
        fields = fields.push(text("No options for this command.").size(14));
    }

    fields.into()
}
