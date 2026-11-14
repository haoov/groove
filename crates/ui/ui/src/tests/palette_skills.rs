//! The skills the palette offers a session, converting to a task once a source.

use groove_controllers::AppState;
use groove_types::Timestamp;

use crate::palette::entries;
use crate::tests::screen::sourced;

/// The fixture's session, with the skills it can be sent.
fn offering(skills: Vec<groove_types::Skill>) -> groove_controllers::AppState {
    let mut app = crate::tests::screen::asking(Vec::new());
    app.skill.skills = skills;
    app
}

fn skill(name: &str, label: &str, kinds: &[&str]) -> groove_types::Skill {
    groove_types::Skill {
        id: format!("groove:{name}"),
        plugin: "groove".into(),
        name: name.into(),
        description: "What it does.".into(),
        hint: String::new(),
        label: label.into(),
        kinds: kinds.iter().map(|one| one.to_string()).collect(),
        resources: Vec::new(),
        editable: false,
        enabled: true,
        changed_at: Timestamp::new(0),
    }
}

/// The palette's skill rows, by their label.
fn offered(app: &AppState) -> Vec<String> {
    let skills = entries(app).into_iter().filter(|one| one.group == "Skills");
    skills.map(|one| one.label).collect()
}

#[test]
fn a_conversion_files_in_each_source_that_is_set_up() {
    let app = sourced(
        offering(vec![skill("convert-explorer", "convert to task", &[])]),
        true,
        true,
    );
    let offered = offered(&app);
    assert_eq!(
        offered,
        ["convert to task in GitHub", "convert to task in Notion"],
        "{offered:?}"
    );
}

#[test]
fn filing_a_task_and_sharing_a_fact_stay_off_the_palette() {
    let app = sourced(
        offering(vec![
            skill("create-task", "create task", &[]),
            skill("promote-fact", "share a fact", &[]),
            skill("convert-explorer", "convert to task", &[]),
        ]),
        true,
        true,
    );
    let offered = offered(&app);
    assert_eq!(offered.len(), 2, "two sources, one skill: {offered:?}");
    assert!(
        offered
            .iter()
            .all(|one| one.starts_with("convert to task in ")),
        "{offered:?}"
    );
}

#[test]
fn one_source_files_without_naming_it() {
    let app = sourced(
        offering(vec![skill("convert-explorer", "convert to task", &[])]),
        false,
        true,
    );
    assert_eq!(offered(&app), ["convert to task"]);
}

#[test]
fn a_session_is_offered_its_own_kind_s_skills() {
    let app = offering(vec![
        skill("save-task", "save task", &["task"]),
        skill("convert-explorer", "convert to task", &["explorer"]),
    ]);
    assert_eq!(
        offered(&app),
        ["convert to task"],
        "an explorer is offered the explorer's own"
    );
}
