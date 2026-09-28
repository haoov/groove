//! What a click on Settings does, from opening it to signing in.

use groove_controllers::{Command, config};

use crate::Ui;
use crate::hit::Target;
use crate::views::settings::Draft;

pub(super) fn acted(target: &Target, ui: &mut Ui) -> Option<Vec<Command>> {
    let asked = match target {
        Target::SettingsOpen => return Some(ui.open_settings()),
        Target::SettingsBack => return Some(ui.close_settings()),
        Target::SettingsCheck => config::Command::CheckEnvironment,
        Target::SettingsLogin => config::Command::Login { cols: 80, rows: 24 },
        Target::SettingsLoginEnd => config::Command::EndLogin,
        _ => return sourced(target, ui).or_else(|| chosen(target, ui)),
    };
    Some(vec![Command::Config(asked)])
}

/// A task source turned on through its fields, or off once confirmed.
fn sourced(target: &Target, ui: &mut Ui) -> Option<Vec<Command>> {
    let settings = &mut ui.settings;
    match target {
        Target::SettingsTurnOn(source) => {
            settings.draft = Some(Draft::of(*source));
            settings.typing = false;
        }
        Target::SettingsDraftField(at) => {
            settings.typing = false;
            if let Some(draft) = settings.draft.as_mut() {
                draft.at = Some(*at);
            }
        }
        Target::SettingsConnect => {
            let draft = settings.draft.as_mut()?;
            draft.at = None;
            return Some(vec![draft.command()]);
        }
        Target::SettingsDraftCancel => settings.draft = None,
        Target::SettingsTurnOff(source) => settings.leaving = Some(*source),
        Target::SettingsTurnOffSure(source) => {
            settings.leaving = None;
            return Some(vec![Command::Config(config::Command::TurnOff(*source))]);
        }
        _ => return None,
    }
    Some(Vec::new())
}

fn chosen(target: &Target, ui: &mut Ui) -> Option<Vec<Command>> {
    let settings = &mut ui.settings;
    match target {
        Target::SettingsSection(section) => {
            settings.section = *section;
            settings.scroll = 0.0;
            settings.search.clear();
            settings.typing = false;
        }
        Target::SettingsSearch => {
            settings.typing = true;
            if let Some(draft) = settings.draft.as_mut() {
                draft.at = None;
            }
        }
        Target::Login => settings.typing = false,
        Target::SetPreference(one) => {
            let set = config::Command::SetPreference(*one);
            return Some(vec![Command::Config(set)]);
        }
        _ => return None,
    }
    Some(Vec::new())
}
