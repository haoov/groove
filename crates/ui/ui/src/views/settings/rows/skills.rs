//! The Agent section's skills: the core ones always on, the user's own and the shared ones switchable.

use groove_controllers::AppState;
use groove_controllers::agent_service::skills::{CORE, USER};
use groove_types::Skill;

use super::super::SettingsUi;
use super::{Row, Section, Value, grouped};

pub(super) fn skills(app: &AppState, settings: &SettingsUi) -> Vec<Row> {
    let of = |plugin: fn(&str) -> bool| -> Vec<Row> {
        let held = app.agent.skills.iter().filter(|one| plugin(&one.plugin));
        held.map(|one| row(one, settings)).collect()
    };
    let mut out = grouped("Core skills", of(|plugin| plugin == CORE));
    out.extend(grouped("Your skills", of(|plugin| plugin == USER)));
    if app.agent.shared.is_some() {
        out.extend(grouped(
            "Shared skills",
            of(|plugin| plugin != CORE && plugin != USER),
        ));
    }
    out
}

fn row(one: &Skill, settings: &SettingsUi) -> Row {
    let said = match one.hint.is_empty() {
        true => one.description.clone(),
        false => one.hint.clone(),
    };
    let value = Value::Skill {
        id: one.id.clone(),
        on: (one.plugin != CORE).then_some(one.enabled),
        said,
        deletes: one.editable,
        asking: one.editable && settings.deleting.as_deref() == Some(one.id.as_str()),
    };
    Row::new(Section::Agent, one.id.clone(), "skill skills plugin", value)
}
