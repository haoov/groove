use groove_gfx::Fonts;
use groove_types::SessionId;

use crate::hit::Target;
use crate::input::Key;
use crate::tests::{CHORD, click, full_app, press, window};
use crate::views::session::Tab;
use crate::{Focus, Ui, view};

fn on_diff(app: &groove_controllers::AppState) -> (Ui, crate::Hits) {
    let mut ui = Ui::default();
    ui.session.tab = Tab::File;
    let (_, hits) = view(app, &ui, window(), &mut Fonts::embedded());
    (ui, hits)
}

#[test]
fn a_click_hands_the_keyboard_to_the_pane_it_lands_in() {
    let app = full_app();
    let (mut ui, hits) = on_diff(&app);
    assert_eq!(ui.focus, Focus::Agent, "the agent has it at the start");

    let rail = hits
        .rect_of(&Target::Session(SessionId::new("a")))
        .expect("a rail row");
    click(rail, &mut ui, &app, &hits);
    assert_eq!(ui.focus, Focus::Rail);

    let agent = hits.rect_of(&Target::Agent).expect("the agent pane");
    click(agent, &mut ui, &app, &hits);
    assert_eq!(ui.focus, Focus::Agent);
}

#[test]
fn the_chord_walks_the_panes_left_to_right() {
    let app = full_app();
    let mut ui = Ui {
        focus: Focus::Rail,
        ..Ui::default()
    };
    for expected in [
        Focus::Agent,
        Focus::Workspace,
        Focus::Sidebar,
        Focus::Sidebar,
    ] {
        press(Key::Right, CHORD, &mut ui, &app);
        assert_eq!(ui.focus, expected, "right stops at the last pane");
    }
    for expected in [Focus::Workspace, Focus::Agent, Focus::Rail, Focus::Rail] {
        press(Key::Left, CHORD, &mut ui, &app);
        assert_eq!(ui.focus, expected, "and left at the first");
    }
}

#[test]
fn a_key_reaches_the_agent_only_while_the_agent_has_the_keyboard() {
    let app = full_app();
    let mut ui = Ui::default();
    let typed = press(
        Key::Char('a'),
        crate::input::Modifiers::default(),
        &mut ui,
        &app,
    );
    assert_eq!(typed.len(), 1, "the agent hears it");
    assert_eq!(typed[0].id(), "agent.send");

    ui.focus = Focus::Workspace;
    let quiet = press(
        Key::Char('a'),
        crate::input::Modifiers::default(),
        &mut ui,
        &app,
    );
    assert!(quiet.is_empty(), "the workspace does not pass it on");
}

#[test]
fn the_rail_walks_its_sessions() {
    let app = full_app();
    let mut ui = Ui {
        focus: Focus::Rail,
        ..Ui::default()
    };
    let down = press(Key::Down, crate::input::Modifiers::default(), &mut ui, &app);
    assert_eq!(down.len(), 1);
    assert_eq!(down[0].id(), "session.select");
}
