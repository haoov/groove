//! A draft being typed in Settings: one row a field, then the row that sends it.

use groove_ui_kit::base::style::Role;
use groove_ui_kit::widgets::Field;

use super::super::draft::{Ask, Draft, asks};
use super::{Row, Section, Value};
use crate::hit::Target;

/// Each field of the draft as the row it is typed in: masked when secret, its hint while empty.
pub(super) fn fields(draft: &Draft, section: Section) -> Vec<Row> {
    let fields = asks(draft.source).iter().zip(&draft.fields).enumerate();
    let row = |(at, (ask, field)): (usize, (&Ask, &Field))| {
        let focused = draft.at == Some(at);
        let target = Target::SettingsDraftField(at);
        let value = Value::Input {
            shown: shown(ask, field, focused),
            focused,
            target,
        };
        Row::new(section, ask.label, ask.hint, value)
    };
    fields.map(row).collect()
}

fn shown(ask: &Ask, field: &Field, focused: bool) -> String {
    match (focused, ask.secret, field.is_empty()) {
        (true, true, _) => field.masked(),
        (true, false, _) => field.shown(),
        (false, _, true) => ask.hint.to_string(),
        (false, true, false) => "•".repeat(field.text().chars().count()),
        (false, false, false) => field.text().to_string(),
    }
}

/// What sends the draft: `busy` while it is out, else what the last send came back with.
pub(super) fn sending(section: Section, busy: Option<&str>, refused: Option<&String>) -> Row {
    let (shown, role) = match (busy, refused) {
        (Some(busy), _) => (busy.to_string(), Role::Working),
        (None, Some(why)) => (why.clone(), Role::Bad),
        (None, None) => (String::new(), Role::Muted),
    };
    let act = busy
        .is_none()
        .then_some(("connect", Target::SettingsConnect));
    Row::new(section, "", "connect", Value::State { shown, role, act })
}
