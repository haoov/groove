//! The Agent section's routines: pause them all, how many run at once, each one and its triggers.

use groove_controllers::AppState;
use groove_controllers::agent_service::routines::Listed;
use groove_controllers::config_service::Preference;
use groove_types::{Routine, RoutinesConfig};
use groove_ui_kit::base::style::Role;

use super::super::SettingsUi;
use super::preferences::count;
use super::{Row, Section, Value, grouped};
use crate::hit::Target;

pub(super) fn routines(app: &AppState, settings: &SettingsUi) -> Vec<Row> {
    let held = app.config.preferences();
    let paused = Value::Toggle {
        on: held.routines_paused,
        flip: Preference::RoutinesPaused(!held.routines_paused),
    };
    let mut out = vec![
        row("pause all".into(), paused),
        Row {
            section: Section::Agent,
            ..count(
                "at once",
                "agents routines run",
                (held.routine_cap, 1, 1),
                "agents",
                Preference::RoutineCap,
            )
        },
    ];
    let config = app.config.routines();
    for one in &app.agent.routines {
        out.extend(listed(one, &config, settings));
    }
    grouped("Routines", out)
}

/// One routine, then its triggers while it is on; or why its file is no routine.
fn listed(one: &Listed, config: &RoutinesConfig, settings: &SettingsUi) -> Vec<Row> {
    let routine = match &one.read {
        Ok(routine) => routine,
        Err(why) => {
            let shown = Value::State {
                shown: why.clone(),
                role: Role::Bad,
                act: None,
            };
            return vec![row(one.id.clone().into(), shown)];
        }
    };
    let on = config.on.contains(&one.id);
    let allowing = settings.allowing.as_deref() == Some(one.id.as_str());
    let value = Value::Routine {
        id: one.id.clone(),
        on,
        said: said(routine),
        allowing: allowing.then(|| scope(routine)),
    };
    let mut out = vec![row(one.id.clone().into(), value)];
    if on {
        let quiet = config.quiet.get(&one.id).cloned().unwrap_or_default();
        out.extend(routine.on.iter().map(|trigger| {
            let on = !quiet.contains(trigger);
            let target = Target::TriggerSwitch(one.id.clone(), *trigger, !on);
            row(
                format!("  {}", trigger.name()).into(),
                Value::Switch { on, target },
            )
        }));
    }
    out
}

/// `the skills it uses · what it does`.
fn said(routine: &Routine) -> String {
    let skills = routine.skills.join(", ");
    match (skills.is_empty(), routine.description.is_empty()) {
        (_, true) => skills,
        (true, false) => routine.description.clone(),
        (false, false) => format!("{skills} · {}", routine.description),
    }
}

/// What a routine may do without asking, as the question that switches it on says it.
fn scope(routine: &Routine) -> String {
    match routine.scope.is_empty() {
        true => "nothing but read".into(),
        false => routine.scope.join(", "),
    }
}

fn row(label: std::borrow::Cow<'static, str>, value: Value) -> Row {
    Row {
        section: Section::Agent,
        group: "",
        label,
        words: "routine routines trigger scope",
        value,
    }
}
