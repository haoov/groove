//! A resource's right-click: the skills that act on its kind, each sent with the resource.

use super::fixture::opened;
use super::*;
use crate::Of;
use crate::input::Input;

fn skill(name: &str, resources: &[&str]) -> groove_types::Skill {
    groove_types::Skill {
        id: format!("groove:{name}"),
        plugin: "groove".into(),
        name: name.into(),
        description: "What it does.".into(),
        hint: String::new(),
        label: name.into(),
        kinds: Vec::new(),
        resources: resources.iter().map(|one| one.to_string()).collect(),
        editable: false,
        enabled: true,
        changed_at: groove_types::Timestamp::new(0),
    }
}

/// The menu a right click on `target` opens, if one does.
fn menu_on(app: &AppState, ui: &mut Ui, target: &Target) -> Option<Of> {
    let (_, hits) = drawn(app, ui);
    let rect = hits
        .rect_of(target)
        .unwrap_or_else(|| panic!("{target:?} was drawn"));
    let (x, y) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
    crate::input::handle(Input::Menu { x, y }, ui, app, &hits, window());
    ui.menu().map(|one| one.of.clone())
}

fn skilled(app: &mut AppState) {
    app.skill.skills = vec![
        skill("investigate", &["*"]),
        skill("rollout-check", &["deployment"]),
        skill("save-task", &[]),
    ];
}

const API_0: &str = "context: staging, namespace: paxone, kind: Pod (pods), name: api-0";

#[test]
fn a_resource_row_offers_the_skills_of_its_kind_sent_with_the_resource() {
    let (mut app, mut ui) = listing();
    skilled(&mut app);
    let row = Target::ResourceRow("paxone/api-0".into());
    let Some(Of::Skills { offered, .. }) = menu_on(&app, &mut ui, &row) else {
        panic!("a skills menu");
    };
    let labels: Vec<&str> = offered.iter().map(|one| one.label.as_str()).collect();
    assert_eq!(
        labels,
        ["investigate"],
        "no deployment's skill, no session's skill"
    );
    assert_eq!(offered[0].args.as_deref(), Some(API_0));
}

#[test]
fn an_object_tab_offers_the_same() {
    let (mut app, mut ui) = opened();
    skilled(&mut app);
    let Some(Of::Skills { offered, .. }) = menu_on(&app, &mut ui, &Target::ResourceTab(0)) else {
        panic!("a skills menu");
    };
    assert_eq!(offered[0].id, "groove:investigate");
    assert_eq!(offered[0].args.as_deref(), Some(API_0));
}

#[test]
fn a_resource_no_skill_acts_on_opens_no_menu() {
    let (mut app, mut ui) = listing();
    app.skill.skills = vec![skill("save-task", &[])];
    let row = Target::ResourceRow("paxone/api-0".into());
    assert!(menu_on(&app, &mut ui, &row).is_none());
}
