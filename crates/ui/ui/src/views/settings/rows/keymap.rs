//! The Keymap section's rows: each action under its group, its chords as the button that rebinds it.

use groove_controllers::AppState;
use groove_ui_kit::base::style::Role;

use super::super::SettingsUi;
use super::{Row, Section, Value};
use crate::hit::Target;
use crate::keymap::{Keymap, TABLE, rebinds};

pub(super) fn keymap(app: &AppState, settings: &SettingsUi) -> Vec<Row> {
    let config = app.config.config.as_ref();
    let keymap = Keymap::of(config);
    let row = |spec: &crate::keymap::Spec| {
        let chords: Vec<String> = keymap
            .chords(spec.action)
            .iter()
            .map(ToString::to_string)
            .collect();
        let (shown, role) = match (settings.binding == Some(spec.action), chords.is_empty()) {
            (true, _) => ("press a chord · esc".to_string(), Role::Working),
            (false, true) => ("unbound".to_string(), Role::Muted),
            (false, false) => (chords.join(" · "), Role::Text),
        };
        let unbind = ("reset", Target::SettingsUnbind(spec.action));
        let reset = rebinds(config, spec.action).then_some(unbind);
        Row {
            section: Section::Keymap,
            group: spec.group,
            label: spec.label.into(),
            words: spec.id,
            value: Value::Picker {
                shown,
                role,
                target: Target::SettingsBind(spec.action),
                act: reset,
            },
        }
    };
    TABLE.iter().map(row).collect()
}
