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
fn an_open_run_s_line_is_held_and_its_lines_hang_bare_under_it() {
    let app = runs();
    let mut ui = Ui::default();
    let held = |ui: &Ui| grounds(&app, ui, Styles::glass_chosen);
    let shut = held(&ui);
    ui.rail.open = Some(crate::views::rail::RunKey {
        fold: crate::views::rail::Fold::Event(TimelineKind::Commit),
        newest: Timestamp::new(70),
    });
    assert_eq!(
        held(&ui),
        shut + 1,
        "the run's own line, its three lines bare"
    );
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

#[test]
fn the_scope_tabs_stand_whatever_number_of_sessions_is_open() {
    for open in [0, 1, 2] {
        let mut app = fed();
        app.session.open.truncate(open);
        let (_, hits) = view(&app, &Ui::default(), window(), &mut Fonts::embedded());
        for mine in [true, false] {
            let tab = hits.rect_of(&Target::FeedScope(mine));
            assert!(tab.is_some(), "{open} open: the {mine} tab");
        }
    }
}

#[test]
fn both_rail_headers_light_edge_to_edge_under_the_pointer() {
    let app = fed();
    let ui = Ui::default();
    let layout = crate::layout::Layout::of(window(), &ui);
    let hover = Styles::new(app.config.theme(), Tokens::new(1.0)).glass_hover();
    for (target, window_) in [
        (Target::SessionsFold, layout.sessions),
        (Target::Feed, layout.feed),
    ] {
        let lit = Ui {
            hover: Some(target.clone()),
            ..Ui::default()
        };
        let (frame, hits) = view(&app, &lit, window(), &mut Fonts::embedded());
        let head = hits.rect_of(&target).expect("the header");
        assert_eq!(
            (head.x, head.w),
            (window_.x, window_.w),
            "{target:?} edge to edge"
        );
        let ground = frame.layers()[0]
            .quads
            .iter()
            .find(|quad| quad.rect == head && quad.color == hover);
        let ground = ground.expect("lit under the pointer");
        assert_eq!(ground.shape.radius, 0.0, "{target:?} square");
    }
}
