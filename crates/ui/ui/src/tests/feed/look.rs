//! How the feed stands on the rail's glass: its scope tabs, its open runs, its errors.

use groove_gfx::{Color, Fonts};
use groove_types::{TimelineKind, Timestamp};
use groove_ui_kit::base::style::{Role, Styles};
use groove_ui_kit::base::tokens::Tokens;

use super::{fed, runs};
use crate::hit::Target;
use crate::tests::{click, window};
use crate::{Ui, view};

/// The quads in `color` the feed drew.
fn grounds(app: &groove_controllers::AppState, ui: &Ui, color: impl Fn(&Styles) -> Color) -> usize {
    let (frame, _) = view(app, ui, window(), &mut Fonts::embedded());
    let feed = crate::layout::Layout::of(window(), ui).feed;
    let color = color(&Styles::new(app.config.theme(), Tokens::new(1.0)));
    frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == color && quad.rect.y >= feed.y && quad.rect.x < feed.right())
        .count()
}

#[test]
fn the_scope_tabs_pick_the_session_in_hand_or_every_session() {
    let app = fed();
    let mut ui = Ui::default();
    for (mine, said) in [
        (true, "narrowed"),
        (true, "and kept so"),
        (false, "widened"),
    ] {
        let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
        let tab = hits.rect_of(&Target::FeedScope(mine)).expect("the tab");
        click(tab, &mut ui, &app, &hits);
        assert_eq!(ui.rail.mine, mine, "{said}");
    }
}

#[test]
fn an_open_run_and_its_lines_stand_on_one_held_ground() {
    let app = runs();
    let mut ui = Ui::default();
    let held = |ui: &Ui| grounds(&app, ui, Styles::glass_chosen);
    let shut = held(&ui);
    ui.rail.open = Some(crate::views::rail::RunKey {
        fold: crate::views::rail::Fold::Event(TimelineKind::Commit),
        newest: Timestamp::new(70),
    });
    assert_eq!(held(&ui), shut + 4, "the run's row, then its three lines");
}

#[test]
fn an_error_lies_on_its_colour_s_tint() {
    let mut app = fed();
    let tint = |styles: &Styles| styles.row_tint(Role::Bad);
    assert_eq!(grounds(&app, &Ui::default(), tint), 0);
    let failed = groove_types::Error::new(groove_types::ErrorKind::Internal, "the walk failed");
    app.errors.push(groove_controllers::Told::now(failed));
    assert_eq!(grounds(&app, &Ui::default(), tint), 1);
}
