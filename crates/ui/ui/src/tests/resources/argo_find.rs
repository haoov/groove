//! An Application's resources find bar: the find key opens it, its words narrow the rows by name.

use crate::input::{Key, Modifiers};
use crate::tests::{press, typed};

use super::argo::failing;
use super::argo_sync::{managed, opened_application};
use super::fixture::runs;

#[test]
fn the_find_key_opens_the_bar_and_its_words_narrow_the_rows_by_name() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.operation = None;
    part.resources = vec![
        managed("Deployment", "billing-web", "OutOfSync", 0),
        managed("Deployment", "billing-api", "OutOfSync", 0),
    ];
    let (app, mut ui) = opened_application(object);
    ui.focus = crate::Focus::Workspace;
    let ctrl = Modifiers {
        ctrl: true,
        ..Default::default()
    };
    press(Key::Char('f'), ctrl, &mut ui, &app);
    let argo = &ui.session.resources.tab().expect("its tab").argo;
    assert!(argo.finding && argo.typing);
    typed("web", &mut ui, &app);
    let shown: Vec<String> = runs(&app, &ui).into_iter().map(|one| one.text).collect();
    assert!(shown.iter().any(|one| one == "billing-web"), "{shown:?}");
    assert!(!shown.iter().any(|one| one == "billing-api"), "{shown:?}");
    assert!(
        shown.iter().any(|one| one == "out of sync · 2"),
        "a filter counts what it holds, the bar aside: {shown:?}"
    );
    press(Key::Escape, Modifiers::default(), &mut ui, &app);
    let argo = &ui.session.resources.tab().expect("its tab").argo;
    assert!(argo.finding && !argo.typing, "a bar with words stays");
}
