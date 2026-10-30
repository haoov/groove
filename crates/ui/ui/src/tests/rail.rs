use std::time::Duration;

use groove_gfx::{Fonts, TextRun};
use groove_types::{AgentStatus, SessionActivity, SessionId, Timestamp};

use groove_controllers::AppState;
use groove_controllers::agent_service::Agent;

use crate::hit::Target;
use crate::input::{Delta, Input};
use crate::tests::{WINDOW, app, full_app, handle, metrics, open, render, window};
use crate::{Ui, view};
use groove_ui_kit::text::ago;

/// Every text the rail drew, with where it drew it.
fn rail_texts(app: &AppState, ui: &Ui) -> Vec<TextRun> {
    let rail = crate::layout::Layout::new(
        groove_gfx::Size::new(WINDOW.0, WINDOW.1),
        &groove_ui_kit::base::tokens::Tokens::new(1.0),
        ui.split,
        false,
    )
    .rail;
    let (frame, _) = view(app, ui, window(), &mut Fonts::embedded());
    frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.x < rail.right())
        .cloned()
        .collect()
}

fn working(app: &mut AppState, id: &str, changed_at: i64) {
    stands(app, id, AgentStatus::Working, changed_at);
}

fn stands(app: &mut AppState, id: &str, status: AgentStatus, changed_at: i64) {
    let activity = SessionActivity {
        status,
        tool: None,
        asks: Vec::new(),
        changed_at: Timestamp::new(changed_at),
        seen_at: None,
    };
    let agent = Agent {
        terminal: None,
        stream: None,
        permissions: Vec::new(),
        activity,
        started_at: Timestamp::new(0),
        launch: 0,
    };
    app.agent.agents.push((SessionId::new(id), agent));
}

#[test]
fn how_long_ago_takes_as_few_characters_as_it_can() {
    assert_eq!(ago(Duration::from_secs(0)), "now");
    assert_eq!(ago(Duration::from_secs(59)), "now");
    assert_eq!(ago(Duration::from_secs(60)), "1m");
    assert_eq!(ago(Duration::from_secs(3_599)), "59m");
    assert_eq!(ago(Duration::from_secs(3_600)), "1h");
    assert_eq!(ago(Duration::from_secs(86_399)), "23h");
    assert_eq!(ago(Duration::from_secs(86_400)), "1d");
    assert_eq!(ago(Duration::from_secs(5 * 86_400)), "5d");
    assert_eq!(ago(Duration::from_secs(7 * 86_400)), "1w");
    assert_eq!(ago(Duration::from_secs(29 * 86_400)), "4w");
    assert_eq!(ago(Duration::from_secs(30 * 86_400)), "1mo");
    assert_eq!(ago(Duration::from_secs(364 * 86_400)), "12mo");
    assert_eq!(ago(Duration::from_secs(2411 * 86_400)), "6y");
}

#[test]
fn a_row_says_how_long_the_agent_has_waited() {
    let mut app = full_app();
    working(&mut app, "a", 0);
    let mut metrics = window();
    metrics.now = Timestamp::new(7_200);
    let (texts, _) = render(&app, &Ui::default(), metrics);
    assert!(texts.iter().any(|t| t == "2h"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "working"));
}

#[test]
fn a_title_too_long_for_the_rail_is_cut_with_an_ellipsis() {
    let mut app = app();
    let long = "Harden Groove: CI gates, security fixes, defect fixes, typed errors";
    app.session.open[0].session.title = long.into();
    let ui = Ui::default();
    let runs = rail_texts(&app, &ui);
    let title = runs
        .iter()
        .find(|run| run.text.starts_with("Harden"))
        .expect("the title is drawn");
    assert!(title.text.ends_with('\u{2026}'), "{}", title.text);
    assert!(title.text.len() < long.len());
}

#[test]
fn the_selected_row_stands_inset_on_a_rounded_held_ground_and_the_hovered_one_lit() {
    let app = full_app();
    let mut ui = Ui::default();
    let tokens = groove_ui_kit::base::tokens::Tokens::new(1.0);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), tokens);
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = |id: &str| {
        hits.rect_of(&Target::Session(SessionId::new(id)))
            .expect("the row")
    };
    let (alpha, beta) = (row("a"), row("b"));
    let on = |frame: &groove_gfx::Frame, rect: groove_gfx::Rect, color| {
        frame.layers()[0]
            .quads
            .iter()
            .any(|quad| quad.rect == rect && quad.color == color && quad.shape.radius == tokens.sm)
    };
    let rail = crate::layout::Layout::of(window(), &ui).rail;
    assert!(alpha.x > rail.x, "inset from the rail's side");
    assert!(
        on(&frame, alpha, styles.glass_chosen()),
        "the selected row is held"
    );
    let ruled = frame.layers()[0]
        .quads
        .iter()
        .any(|quad| quad.color == styles.chosen() && quad.rect.right() <= rail.right());
    assert!(!ruled, "no rules on the rail");
    assert!(
        !on(&frame, beta, styles.glass_hover()),
        "an idle row has no ground"
    );
    ui.hover = Some(Target::Session(SessionId::new("b")));
    let (hovered, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        on(&hovered, beta, styles.glass_hover()),
        "the pointer lights its row"
    );
}

#[test]
fn the_wheel_scrolls_the_rows_and_a_row_off_the_top_cannot_be_clicked() {
    let mut app = full_app();
    for n in 0..20 {
        app.session
            .open
            .push(open(&format!("s{n}"), &format!("Session {n}")));
    }
    let mut ui = Ui::default();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let first = Target::Session(SessionId::new("a"));
    let top = hits.rect_of(&first).expect("the first row is drawn");
    handle(
        Input::Scroll {
            x: 0.0,
            y: 0.0,
            delta: Delta::Pixels {
                across: 0.0,
                down: -top.h * 2.0,
            },
        },
        &mut ui,
        &app,
        &hits,
        metrics(WINDOW.0, WINDOW.1, 1.0),
    );
    assert!(ui.rail.scroll > 0.0);
    let (_, scrolled) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        scrolled.rect_of(&first).is_none(),
        "the first row scrolled out of the rail"
    );
}

/// Alpha idle, Beta working, Gamma done and not seen, Delta failed, opened in that order.
fn four_states() -> AppState {
    let mut app = app();
    app.session.open.push(crate::tests::open("c", "Gamma"));
    app.session.open.push(crate::tests::open("d", "Delta"));
    working(&mut app, "b", 0);
    stands(&mut app, "c", AgentStatus::Done { seen: false }, 0);
    let error = AgentStatus::Error {
        message: "exit 101".into(),
    };
    stands(&mut app, "d", error, 0);
    app
}

#[test]
fn each_row_stands_under_its_state_in_the_order_opened() {
    let app = four_states();
    let ui = Ui::default();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let y = |id: &str| {
        let row = hits.rect_of(&Target::Session(SessionId::new(id)));
        row.expect("the row").y
    };
    assert!(y("d") < y("b"), "needs you, then working");
    assert!(y("b") < y("a"), "then idle");
    assert!(y("a") < y("c"), "done stands in idle, in the order opened");
    let texts = crate::tests::all_texts(&frame);
    for heading in ["NEEDS YOU · 1", "WORKING · 1", "IDLE · 2"] {
        assert!(
            texts.iter().any(|one| one == heading),
            "{heading}: {texts:?}"
        );
    }
}

#[test]
fn the_arrows_walk_the_rows_in_the_order_drawn() {
    let mut app = four_states();
    app.session.selected = Some(SessionId::new("b"));
    let mut ui = Ui {
        focus: crate::Focus::Rail,
        ..Ui::default()
    };
    let none = crate::input::Modifiers::default();
    let walked = crate::tests::press(crate::input::Key::Down, none, &mut ui, &app);
    let select = groove_controllers::session::Command::Select {
        session: SessionId::new("a"),
    };
    assert_eq!(walked, [groove_controllers::Command::Session(select)]);
}
