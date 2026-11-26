//! The region that has the keys stands in a thin accent ring; the agent shows them by its caret.

use groove_gfx::{Color, Fonts, Frame, Rect};
use groove_ui_kit::base::style::{Role, Styles};
use groove_ui_kit::base::tokens::Tokens;

use crate::layout::Layout;
use crate::tests::{full_app, window};
use crate::views::session::Tab;
use crate::{Focus, Ui, view};

/// Where a region stands in the layout.
type Around = fn(&Layout) -> Rect;

fn styles(app: &groove_controllers::AppState) -> Styles {
    Styles::new(app.config.theme(), Tokens::new(1.0))
}

/// Every quad of the frame in `color`.
fn in_colour(frame: &Frame, color: Color) -> Vec<Rect> {
    let quads = frame.layers().iter().flat_map(|one| one.quads.iter());
    quads
        .filter(|quad| quad.color == color)
        .map(|quad| quad.rect)
        .collect()
}

#[test]
fn the_ring_stands_around_the_region_that_has_the_keys_and_nowhere_else() {
    let app = full_app();
    let focus = styles(&app).focus();
    let cases: [(Focus, Around); 4] = [
        (Focus::Rail, |one| one.sessions),
        (Focus::Workspace, |one| one.strip),
        (Focus::Sidebar, |one| {
            groove_ui_kit::widgets::aside_bounds(&Tokens::new(1.0), one.sidebar)
        }),
        (Focus::Terminal, |one| one.manual),
    ];
    for (keys, around) in cases {
        let mut ui = Ui {
            focus: keys,
            ..Ui::default()
        };
        ui.session.tab = Tab::Diff;
        ui.session.manual = true;
        let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
        let expected = around(&Layout::of(window(), &ui));
        assert!(!expected.is_empty(), "{keys:?} stands open");
        assert_eq!(
            in_colour(&frame, focus),
            [expected],
            "{keys:?}: one ring, around it"
        );
    }
}

#[test]
fn the_prompt_stands_in_the_ring_while_the_agent_has_the_keys() {
    let mut app = crate::tests::agent::asking(Vec::new());
    crate::tests::agent::headless(&mut app);
    let focus = styles(&app).focus();
    let ringed = |keys: Focus| {
        let ui = Ui {
            surface: crate::Surface::Session,
            focus: keys,
            ..Ui::default()
        };
        let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
        let prompt = hits.rect_of(&crate::hit::Target::Prompt).expect("the prompt");
        let ring = in_colour(&frame, focus);
        ring.iter().any(|one| prompt.contains(one.x, one.y))
    };
    assert!(ringed(Focus::Agent), "the prompt ringed while it has the keys");
    assert!(!ringed(Focus::Rail), "not while another region has them");
}

#[test]
fn a_selection_stays_neutral_in_the_region_that_has_the_keys() {
    let app = full_app();
    let ui = Ui {
        focus: Focus::Rail,
        ..Ui::default()
    };
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits.rect_of(&crate::hit::Target::Session(groove_types::SessionId::new(
        "a",
    )));
    let row = row.expect("the selected row");
    let styles = styles(&app);
    assert!(
        in_colour(&frame, styles.glass_chosen()).contains(&row),
        "held, neutral"
    );
    let accent = styles.color(Role::Accent);
    let lit = frame.layers().iter().flat_map(|one| one.quads.iter());
    assert!(
        !lit.into_iter()
            .any(|quad| quad.rect == row && quad.color.with_alpha(255) == accent.with_alpha(255)),
        "no accent under it"
    );
}
