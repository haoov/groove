//! A routine's session: on the rail under a folded Routines heading, off the board, its agent pane alone.

use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::{SessionId, SessionKind};

use crate::hit::{Hits, Target};
use crate::layout::{Edge, Layout};
use crate::tests::{full_app, handle, open, window};
use crate::{Surface, Ui, view};

/// The full app with a routine's session beside the explorers, on disk too.
fn routed() -> AppState {
    let mut app = full_app();
    let mut digest = open("routine-user-digest", "digest");
    digest.session.kind = SessionKind::Routine {
        routine: "user:digest".into(),
    };
    app.session
        .living
        .push(groove_controllers::session_service::Living {
            session: digest.session.clone(),
            worktrees: Vec::new(),
            repos: 0,
        });
    app.session.open.push(digest);
    app
}

fn drawn(app: &AppState, ui: &Ui) -> (Vec<String>, Hits) {
    let (frame, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let texts = frame.layers()[0]
        .texts
        .iter()
        .map(|run| run.text.clone())
        .collect();
    (texts, hits)
}

#[test]
fn the_rail_holds_routine_sessions_under_a_folded_heading_that_opens_them() {
    let app = routed();
    let mut ui = Ui::default();
    let digest = Target::Session(SessionId::new("routine-user-digest"));
    let (texts, hits) = drawn(&app, &ui);
    assert!(texts.iter().any(|t| t == "ROUTINES · 1"), "{texts:?}");
    assert!(hits.rect_of(&digest).is_none(), "folded at first");
    let heading = hits.rect_of(&Target::Routines).expect("the heading");
    let alpha = hits.rect_of(&Target::Session(SessionId::new("a"))).unwrap();
    assert!(alpha.y < heading.y, "the routines come after the sessions");

    let (x, y) = (heading.x + heading.w / 2.0, heading.y + heading.h / 2.0);
    let press = crate::input::Input::Press {
        x,
        y,
        mods: Default::default(),
    };
    handle(press, &mut ui, &app, &hits, window());
    let (_, hits) = drawn(&app, &ui);
    let row = hits.rect_of(&digest).expect("open, its session shows");
    assert!(row.y > heading.y);
}

#[test]
fn a_routine_session_is_its_agent_pane_alone_and_off_the_board() {
    let mut app = routed();
    app.session.selected = Some(SessionId::new("routine-user-digest"));
    let mut ui = Ui::default();
    ui.settle(&app);
    let layout = Layout::of(window(), &ui);
    assert_eq!(layout.agent.x, layout.rail.right());
    assert_eq!(layout.agent.right(), layout.window.w);
    assert!(layout.workspace.is_empty() && layout.sidebar.is_empty());
    let (texts, hits) = drawn(&app, &ui);
    assert!(!texts.iter().any(|t| t == "overview"), "{texts:?}");
    assert!(hits.rect_of(&Target::Split(Edge::Agent)).is_none());

    ui.surface = Surface::Board;
    let rail = Layout::of(window(), &ui).rail.right();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let texts: Vec<_> = frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.x > rail)
        .map(|run| run.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "Alpha"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "digest"), "{texts:?}");
}

#[test]
fn a_routine_session_s_rail_item_runs_its_routine_unless_a_run_is_on() {
    let mut app = routed();
    let mut ui = Ui::default();
    ui.rail.routines = true;
    let run = Target::RoutineRun("user:digest".into());
    let (_, hits) = drawn(&app, &ui);
    let button = hits.rect_of(&run).expect("its run button");
    let press = crate::input::Input::Press {
        x: button.x + button.w / 2.0,
        y: button.y + button.h / 2.0,
        mods: Default::default(),
    };
    let asked = handle(press, &mut ui, &app, &hits, window());
    let wanted = groove_controllers::agent::Command::RunRoutine {
        id: "user:digest".into(),
    };
    assert_eq!(asked, [groove_controllers::Command::Agent(wanted)]);

    app.agent
        .runs
        .running
        .push(groove_controllers::agent_service::runs::Run {
            routine: "user:digest".into(),
            session: SessionId::new("routine-user-digest"),
            trigger: None,
            about: String::new(),
            sent_at: None,
            went: false,
        });
    let (_, hits) = drawn(&app, &ui);
    assert!(hits.rect_of(&run).is_none(), "running, it has no button");
}
