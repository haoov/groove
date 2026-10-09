//! The Resources tab across sessions.

use groove_controllers::Command;
use groove_controllers::session::Command as Session;
use groove_types::{SessionId, Timestamp};

use super::pods;
use super::tab::{crashing, lands, link, opened};
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
