//! The Resources tab across sessions.

use groove_controllers::Command;
use groove_controllers::session::Command as Session;
use groove_types::{SessionId, Timestamp};

use super::fixture::{crashing, lands, link, opened};
use super::pods;
use crate::hit::Target;
use crate::tests::click;
use crate::tests::settings::drawn;
use crate::views::session::Tab;

#[test]
fn a_session_holding_no_cluster_shows_its_overview_not_the_last_object() {
    let (mut app, mut ui) = opened();
    lands(&mut app, &link("api-0", pods()).key(), vec![crashing()]);
    let (texts, hits) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "CONDITIONS"), "{texts:?}");
    let b = SessionId::new("b");
    let row = hits.rect_of(&Target::Session(b.clone())).expect("b's row");
    let asked = click(row, &mut ui, &app, &hits);
    let select = Command::Session(Session::Open { session: b.clone() });
    assert!(asked.contains(&select), "{asked:?}");
    app.session.select(&b, Timestamp::new(0));
    ui.settle(&app);
    assert_eq!(ui.session.tab, Tab::Overview);
    let (texts, _) = drawn(&app, &ui);
    assert!(!texts.iter().any(|one| one == "CONDITIONS"), "{texts:?}");
}

#[test]
fn each_session_keeps_its_own_object_tabs() {
    let (mut app, mut ui) = opened();
    ui.settle(&app);
    assert_eq!(ui.session.resources.opened.len(), 1, "a's tab");
    let (a, b) = (SessionId::new("a"), SessionId::new("b"));
    app.session.select(&b, Timestamp::new(0));
    ui.settle(&app);
    assert!(
        ui.session.resources.opened.is_empty(),
        "b shows none of a's tabs"
    );
    app.session.select(&a, Timestamp::new(0));
    ui.settle(&app);
    assert_eq!(ui.session.resources.opened.len(), 1, "a's tab comes back");
}
