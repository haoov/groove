//! What a click on Settings does: open it, go back, pick a section, search, set a preference.

use groove_controllers::{Command, config};

use crate::Ui;
use crate::hit::Target;

pub(super) fn acted(target: &Target, ui: &mut Ui) -> Option<Vec<Command>> {
    let settings = &mut ui.settings;
    match target {
        Target::SettingsOpen => {
            settings.open = true;
            ui.overlay = None;
        }
        Target::SettingsBack => {
            settings.open = false;
            settings.typing = false;
        }
        Target::SettingsSection(section) => {
            settings.section = *section;
            settings.search.clear();
            settings.typing = false;
        }
        Target::SettingsSearch => settings.typing = true,
        Target::SetPreference(one) => {
            let set = config::Command::SetPreference(*one);
            return Some(vec![Command::Config(set)]);
        }
        _ => return None,
    }
    Some(Vec::new())
}
