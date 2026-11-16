//! The board's top: no filter and no button, its columns' headings first.

use groove_gfx::Fonts;

use crate::layout::Layout;
use crate::tests::{full_app, window};
use crate::{Surface, Ui, view};

#[test]
fn the_board_opens_straight_on_its_columns_headings() {
    let app = full_app();
    let ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let lists = Layout::of(window(), &ui).lists;
    let texts: Vec<_> = frame
        .layers()
        .iter()
        .flat_map(|one| one.texts.iter())
        .collect();
    assert!(
        !texts.iter().any(|one| one.text.starts_with("filter")),
        "no filter"
    );
    let live = texts
        .iter()
        .find(|one| one.text.starts_with("LIVE"))
        .expect("the live column's heading");
    let tokens = groove_ui_kit::base::tokens::Tokens::new(1.0);
    assert!(
        live.y >= lists.y && live.y < lists.y + tokens.header,
        "at the top of its columns' room"
    );
}

#[test]
fn a_board_item_stands_on_a_sunk_card_as_the_overview_s_do() {
    let mut app = full_app();
    app.task.tasks = vec![crate::tests::task(
        "gh-a-b-1",
        "waiting one",
        "github.com/a/b#1",
    )];
    let ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&crate::hit::Target::Task("gh-a-b-1".into()))
        .expect("the item");
    let styles = groove_ui_kit::base::style::Styles::new(
        app.config.theme(),
        groove_ui_kit::base::tokens::Tokens::new(1.0),
    );
    let quads = frame.layers().iter().flat_map(|one| one.quads.iter());
    let card = quads
        .clone()
        .find(|q| q.color == styles.band() && q.rect.y >= row.y && q.rect.bottom() <= row.bottom());
    let card = card.expect("the band's ground");
    assert!(
        quads
            .clone()
            .any(|q| q.rect == card.rect && q.shape.stroke > 0.0),
        "edged by a hairline"
    );
}

#[test]
fn the_board_keeps_the_agent_s_room_and_with_the_rail_folded_takes_its_width() {
    let ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    let open = Layout::of(window(), &ui);
    assert_eq!(
        (open.lists.y, open.lists.h),
        (open.agent.y, open.agent.h),
        "the bar's room above and below, as the agent keeps"
    );
    let mut folded = ui;
    (folded.rail.stowed, folded.rail.folded) = (true, true);
    let folded = Layout::of(window(), &folded);
    assert_eq!(folded.lists.x, folded.window.x, "from the window's edge");
    assert_eq!(
        folded.lists.right(),
        open.lists.right(),
        "to where it ended"
    );
}
