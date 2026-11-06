//! Converting a session to a task from the palette: one entry for each source that is set up.

use super::*;

/// The skills the palette offers, by label.
fn offered(app: &groove_controllers::AppState) -> Vec<String> {
    let all = crate::palette::entries(app).into_iter();
    all.filter(|one| one.group == "Skills")
        .map(|one| one.label)
        .collect()
}

#[test]
fn a_conversion_files_in_each_source_that_is_set_up() {
    let app = sourced(
        offering(vec![skill("convert-explorer", "convert to task", &[])]),
        true,
        true,
    );
    let labels = offered(&app);
    for one in ["convert to task in GitHub", "convert to task in Notion"] {
        assert!(labels.iter().any(|label| label == one), "{labels:?}");
    }
    assert!(
        !labels.iter().any(|one| one == "convert to task"),
        "{labels:?}"
    );
    let open = app.session.selected().expect("a session");
    let sources = groove_controllers::task_service::source_ids(app.config.config.as_ref());
    let first = app.skill.offers(&open.session.kind, &sources).remove(0);
    assert_eq!(first.id, "groove:convert-explorer");
    assert_eq!(first.args.as_deref(), Some("github"));
}

#[test]
fn filing_a_task_and_sharing_a_fact_stay_off_the_menu_and_converting_names_each_source() {
    let app = sourced(
        offering(vec![
            skill("create-task", "create task", &[]),
            skill("promote-fact", "share a fact", &[]),
            skill("convert-explorer", "convert to task", &[]),
        ]),
        true,
        true,
    );
    let labels = offered(&app);
    let rows: Vec<&String> = labels.iter().filter(|one| one.contains(" in ")).collect();
    let mut once = rows.clone();
    once.sort();
    once.dedup();
    assert_eq!(rows.len(), 2, "two sources, one skill: {rows:?}");
    assert!(
        !labels
            .iter()
            .any(|one| one.starts_with("create task") || one.starts_with("share a fact")),
        "{labels:?}"
    );
    assert_eq!(once.len(), rows.len(), "no entry says what another says");
}

#[test]
fn one_source_files_without_naming_it() {
    let app = sourced(
        offering(vec![skill("convert-explorer", "convert to task", &[])]),
        false,
        true,
    );
    let labels = offered(&app);
    assert!(
        labels.iter().any(|one| one == "convert to task"),
        "{labels:?}"
    );
}
