//! The sessions' window folded to its header by a click on it, the feed rising under it.

use groove_gfx::Fonts;

use crate::hit::Target;
use crate::input::Key;
use crate::layout::{Edge, Layout};
use crate::tests::{ALT, click, full_app, press, window};
use crate::{Ui, view};

fn header_click(ui: &mut Ui, app: &groove_controllers::AppState) {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let head = hits.rect_of(&Target::SessionsFold).expect("the header");
    let acted = click(head, ui, app, &hits);
    assert!(acted.is_empty(), "it only folds: {acted:?}");
}

#[test]
fn a_click_on_the_sessions_header_folds_the_window_to_it_and_the_feed_keeps_its_height() {
    let app = full_app();
    let mut ui = Ui::default();
    let open = Layout::of(window(), &ui);
    header_click(&mut ui, &app);
    assert!(ui.rail.stowed);
    let folded = Layout::of(window(), &ui);
    let tokens = window().tokens();
    assert_eq!(folded.sessions.h, tokens.bar, "the header alone");
    assert_eq!(folded.sessions.y, open.sessions.y, "where it stood");
    assert_eq!(folded.feed, open.feed, "the feed as it stood");

    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(hits.rect_of(&Target::Board).is_none(), "no Board row");
    let row = Target::Session(groove_types::SessionId::new("a"));
    assert!(hits.rect_of(&row).is_none(), "no session row");
    let edge = Target::Split(Edge::Feed);
    assert!(
        hits.rect_of(&edge).is_some(),
        "the feed's top still dragged"
    );

    header_click(&mut ui, &app);
    assert!(!ui.rail.stowed);
    assert_eq!(Layout::of(window(), &ui), open, "opened as it was");
}

#[test]
fn both_folded_the_rail_is_its_two_headers() {
    let mut ui = Ui::default();
    ui.rail.stowed = true;
    ui.rail.folded = true;
    let layout = Layout::of(window(), &ui);
    let tokens = window().tokens();
    assert_eq!((layout.sessions.h, layout.feed.h), (tokens.bar, tokens.bar));
    assert!(
        layout.feed.y > layout.sessions.bottom() + tokens.sm,
        "the feed stays at the foot"
    );
}

#[test]
fn the_rail_s_chord_opens_the_folded_sessions_and_hands_them_the_keys() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.rail.stowed = true;
    press(Key::Char('r'), ALT, &mut ui, &app);
    assert!(!ui.rail.stowed);
    assert_eq!(ui.focus, crate::Focus::Rail);
}

#[test]
fn with_both_rail_windows_folded_the_agent_takes_the_whole_width() {
    let app = full_app();
    let mut ui = Ui::default();
    let open = Layout::of(window(), &ui);
    ui.rail.stowed = true;
    let half = Layout::of(window(), &ui);
    assert_eq!(
        half.agent, open.agent,
        "one window open: the rail keeps its column"
    );
    ui.rail.folded = true;
    let folded = Layout::of(window(), &ui);
    assert_eq!(folded.agent.x, folded.window.x, "from the window's edge");
    assert_eq!(
        folded.agent.right(),
        open.agent.right(),
        "to where it ended"
    );
    assert_eq!(
        (folded.agent.y, folded.agent.h),
        (open.agent.y, open.agent.h)
    );
    assert!(
        folded.sessions.bottom() <= folded.agent.y,
        "the sessions' header over it"
    );
    assert!(
        folded.feed.y >= folded.agent.bottom(),
        "the feed's header under it"
    );

    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        hits.rect_of(&Target::Split(Edge::Rail)).is_none(),
        "the rail's edge is gone"
    );
    let x = folded.agent.x + folded.agent.w / 8.0;
    let y = folded.agent.y + folded.agent.h / 2.0;
    assert!(x < open.rail.right(), "inside the rail's column");
    assert_eq!(
        hits.at(x, y),
        Some(Target::Agent),
        "the agent's screen answers there"
    );
}

#[test]
fn the_rail_s_chord_on_the_rail_folds_the_sessions_and_hands_the_keys_to_the_agent() {
    let app = full_app();
    let mut ui = Ui {
        focus: crate::Focus::Rail,
        ..Ui::default()
    };
    press(Key::Char('r'), ALT, &mut ui, &app);
    assert!(ui.rail.stowed, "folded");
    assert_eq!(ui.focus, crate::Focus::Agent);
    press(Key::Char('r'), ALT, &mut ui, &app);
    assert!(!ui.rail.stowed, "and open again");
    assert_eq!(ui.focus, crate::Focus::Rail);
}
