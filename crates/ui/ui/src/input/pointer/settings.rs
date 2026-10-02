//! What a click on Settings does, from opening it to signing in.

use groove_controllers::{AppState, Command, agent, config};

use crate::hit::{Hits, Target};
use crate::keymap;
use crate::views::settings::Draft;
use crate::views::settings::rows::choices;
use crate::{Corner, Menu, Of, Overlay, Ui};

pub(super) fn acted(
    target: &Target,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
) -> Option<Vec<Command>> {
    let asked = match target {
        Target::SettingsPick(source, slot) => {
            let under = hits.rect_of(target)?;
            ui.overlay = Some(Overlay::Menu(Menu {
                at: (under.x, under.bottom()),
                corner: Corner::TopLeft,
                of: Of::Mapping(choices(app, *source, *slot)),
            }));
            return Some(Vec::new());
        }
        Target::SettingsBind(action) => {
            ui.settings.binding = Some(*action);
            return Some(Vec::new());
        }
        Target::SettingsUnbind(action) => {
            config::Command::Rebind(keymap::reset(app.config.config.as_ref(), *action))
        }
        Target::SettingsOpen => return Some(ui.open_settings()),
        Target::SettingsBack => return Some(ui.close_settings()),
        Target::SettingsCheck => config::Command::CheckEnvironment,
        Target::SettingsLogin => config::Command::Login { cols: 80, rows: 24 },
        Target::SettingsLoginEnd => config::Command::EndLogin,
        _ => {
            let picked = sourced(target, ui).or_else(|| skilled(target, ui));
            let picked = picked.or_else(|| routined(target, ui));
            return picked.or_else(|| chosen(target, ui));
        }
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
        Target::SettingsShare => {
            settings.draft = Some(Draft::shared());
            settings.typing = false;
        }
        Target::SettingsUnshare => settings.unsharing = true,
        Target::SettingsUnshareSure => {
            settings.unsharing = false;
            settings.draft = None;
            return Some(vec![Command::Config(config::Command::LeaveShared)]);
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

/// A skill switched, or one of the user's own deleted once confirmed.
fn skilled(target: &Target, ui: &mut Ui) -> Option<Vec<Command>> {
    let settings = &mut ui.settings;
    let asked = match target {
        Target::SkillSwitch(id, on) => agent::Command::SwitchSkill {
            id: id.clone(),
            on: *on,
        },
        Target::SkillDelete(id) => {
            settings.deleting = Some(id.clone());
            return Some(Vec::new());
        }
        Target::SkillDeleteKeep => {
            settings.deleting = None;
            return Some(Vec::new());
        }
        Target::SkillDeleteSure(id) => {
            settings.deleting = None;
            let name = id.strip_prefix("user:")?.to_string();
            agent::Command::DeleteSkill { name }
        }
        _ => return None,
    };
    Some(vec![Command::Agent(asked)])
}

/// A routine switched on once its scope is allowed, or off; one of its triggers switched.
fn routined(target: &Target, ui: &mut Ui) -> Option<Vec<Command>> {
    let settings = &mut ui.settings;
    let asked = match target {
        Target::RoutineOn(id) => {
            settings.allowing = Some(id.clone());
            return Some(Vec::new());
        }
        Target::RoutineKeep => {
            settings.allowing = None;
            return Some(Vec::new());
        }
        Target::RoutineAllow(id) => {
            settings.allowing = None;
            config::Command::SwitchRoutine {
                id: id.clone(),
                on: true,
            }
        }
        Target::RoutineOff(id) => config::Command::SwitchRoutine {
            id: id.clone(),
            on: false,
        },
        Target::TriggerSwitch(id, trigger, on) => config::Command::SwitchTrigger {
            id: id.clone(),
            trigger: *trigger,
            on: *on,
        },
        _ => return None,
    };
    Some(vec![Command::Config(asked)])
}
