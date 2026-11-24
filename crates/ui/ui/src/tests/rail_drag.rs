//! A session carried across the rail by hand, or placed from its row's menu; a section folded.

use groove_controllers::{AppState, Command, session};
use groove_types::{Section, SessionId};

use crate::hit::Target;
use crate::tests::{app, drag_at, drawn, open, pressed, release};
use crate::{Overlay, Ui};

/// Delta, Gamma, Beta, Alpha from the top: all active, unranked, the last opened first.
fn four() -> AppState {
    let mut app = app();
    app.session.open.push(open("c", "Gamma"));
    app.session.open.push(open("d", "Delta"));
    app
}

fn place(id: &str, section: Section, index: usize) -> Vec<Command> {
    vec![Command::Session(session::Command::Place {
        session: SessionId::new(id),
        section,
        index,
    })]
}

fn row(hits: &crate::hit::Hits, id: &str) -> groove_gfx::Rect {
    hits.rect_of(&Target::Session(SessionId::new(id)))
        .expect("the row")
}

#[test]
fn a_row_dropped_lower_in_its_section_lands_there() {
    let app = four();
    let mut ui = Ui::default();
    let (_, hits) = drawn(&app, &ui);
    let (delta, beta) = (row(&hits, "d"), row(&hits, "b"));
    let x = delta.x + delta.w / 2.0;
    pressed(x, delta.y + delta.h / 2.0, &mut ui, &app, &hits);
    drag_at(x, beta.bottom(), &mut ui, &app, &hits);
    let landed = release(&mut ui, &app, &hits);
    assert_eq!(landed, place("d", Section::Active, 2));
}

#[test]
fn a_row_dropped_under_the_paused_heading_is_paused() {
    let app = four();
    let mut ui = Ui::default();
    let (_, hits) = drawn(&app, &ui);
    let (beta, alpha) = (row(&hits, "b"), row(&hits, "a"));
    let x = beta.x + beta.w / 2.0;
    pressed(x, beta.y + beta.h / 2.0, &mut ui, &app, &hits);
    let below = alpha.bottom() + alpha.h;
    drag_at(x, below, &mut ui, &app, &hits);
    let landed = release(&mut ui, &app, &hits);
    assert_eq!(landed, place("b", Section::Paused, 0));
}

#[test]
fn a_press_that_does_not_move_places_nothing() {
    let app = four();
    let mut ui = Ui::default();
    let (_, hits) = drawn(&app, &ui);
    let beta = row(&hits, "b");
    let (x, y) = (beta.x + beta.w / 2.0, beta.y + beta.h / 2.0);
    pressed(x, y, &mut ui, &app, &hits);
    drag_at(x, y + 1.0, &mut ui, &app, &hits);
    assert!(release(&mut ui, &app, &hits).is_empty());
}

#[test]
fn a_row_s_menu_pauses_its_session_and_a_paused_one_s_resumes_it() {
    let mut app = four();
    let mut ui = Ui::default();
    let (_, hits) = drawn(&app, &ui);
    let beta = row(&hits, "b");
    let (x, y) = (beta.x + beta.w / 2.0, beta.y + beta.h / 2.0);
    let menu = crate::input::Input::Menu { x, y };
    crate::tests::handle(menu, &mut ui, &app, &hits, crate::tests::window());
    assert!(matches!(ui.overlay, Some(Overlay::Menu(_))));
    let picked = crate::tests::hit(&Target::MenuRow(0), &mut ui, &app);
    assert_eq!(picked, place("b", Section::Paused, 0));

    app.session
        .get_mut(&SessionId::new("b"))
        .unwrap()
        .state
        .section = Section::Paused;
    let (_, hits) = drawn(&app, &ui);
    let beta = row(&hits, "b");
    let (x, y) = (beta.x + beta.w / 2.0, beta.y + beta.h / 2.0);
    let menu = crate::input::Input::Menu { x, y };
    crate::tests::handle(menu, &mut ui, &app, &hits, crate::tests::window());
    let picked = crate::tests::hit(&Target::MenuRow(0), &mut ui, &app);
    assert_eq!(picked, place("b", Section::Active, 0));
}

#[test]
fn a_section_s_heading_folds_its_sessions_away_and_opens_them_again() {
    let app = four();
    let mut ui = Ui::default();
    let active = Target::SectionFold(Section::Active);
    crate::tests::hit(&active, &mut ui, &app);
    let (_, hits) = drawn(&app, &ui);
    assert!(hits.rect_of(&active).is_some());
    assert!(
        hits.rect_of(&Target::Session(SessionId::new("d")))
            .is_none()
    );
    assert!(crate::views::rail::shown(&app, ui.rail.shut).is_empty());

    crate::tests::hit(&active, &mut ui, &app);
    let (_, hits) = drawn(&app, &ui);
    assert!(
        hits.rect_of(&Target::Session(SessionId::new("d")))
            .is_some()
    );
}
