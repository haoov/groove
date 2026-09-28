//! What a click on Settings does, from opening it to signing in.

use groove_controllers::{Command, config};

use crate::Ui;
use crate::hit::Target;

pub(super) fn acted(target: &Target, ui: &mut Ui) -> Option<Vec<Command>> {
    let asked = match target {
        Target::SettingsOpen => return Some(ui.open_settings()),
        Target::SettingsBack => return Some(ui.close_settings()),
        Target::SettingsCheck => config::Command::CheckEnvironment,
        Target::SettingsLogin => config::Command::Login { cols: 80, rows: 24 },
        Target::SettingsLoginEnd => config::Command::EndLogin,
        _ => return chosen(target, ui),
    };
    Some(vec![Command::Config(asked)])
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
        Target::SettingsSearch => settings.typing = true,
        Target::Login => settings.typing = false,
        Target::SetPreference(one) => {
            let set = config::Command::SetPreference(*one);
            return Some(vec![Command::Config(set)]);
        }
        _ => return None,
    }
    Some(Vec::new())
}
