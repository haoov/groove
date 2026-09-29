use std::time::Duration;

use groove_gfx::{Fonts, TextRun};
use groove_types::{AgentStatus, SessionActivity, SessionId, Timestamp};

use groove_controllers::AppState;
use groove_controllers::agent_service::Agent;

use crate::hit::Target;
use crate::input::{Delta, Input};
use crate::tests::{WINDOW, app, full_app, handle, metrics, open, window};
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
    let activity = SessionActivity {
        status: AgentStatus::Working,
        tool: None,
        asks: Vec::new(),
        auto_approve: false,
        changed_at: Timestamp::new(changed_at),
        seen_at: None,
    };
    let agent = Agent {
        terminal: None,
        activity,
        started_at: Timestamp::new(0),
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
}

#[test]
fn a_row_says_how_long_the_agent_has_waited() {
    let mut app = full_app();
    working(&mut app, "a", 0);
    let mut metrics = window();
    metrics.now = Timestamp::new(7_200);
    let (frame, _) = view(&app, &Ui::default(), metrics, &mut Fonts::embedded());
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|run| run.text.clone())
        .collect();
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
fn the_selected_row_is_ruled_in_the_accent_and_the_hovered_row_lit_as_in_any_list() {
    let app = full_app();
    let mut ui = Ui::default();
    let styles = groove_ui_kit::base::style::Styles::new(
        app.config.theme(),
        groove_ui_kit::base::tokens::Tokens::new(1.0),
    );
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let alpha = hits
        .rect_of(&Target::Session(SessionId::new("a")))
        .expect("the selected row is drawn");
    let beta = hits
        .rect_of(&Target::Session(SessionId::new("b")))
        .expect("the other row is drawn");
    let raised = |frame: &groove_gfx::Frame, rect: groove_gfx::Rect| {
        frame.layers()[0]
            .quads
            .iter()
            .any(|quad| quad.rect == rect && quad.color == styles.hover())
    };
    let ruled = |frame: &groove_gfx::Frame, rect: groove_gfx::Rect| {
        let rules = frame.layers()[0].quads.iter().filter(|quad| {
            quad.color == styles.chosen() && quad.rect.x == rect.x && quad.rect.w == rect.w
        });
        rules
            .filter(|quad| quad.rect.y == rect.y || quad.rect.bottom() == rect.bottom())
            .count()
    };
    assert_eq!(
        ruled(&frame, alpha),
        2,
        "the selected session is ruled above and below"
    );
    assert!(!raised(&frame, alpha), "and carries no background");
    assert_eq!(ruled(&frame, beta), 0);
    assert!(!raised(&frame, beta), "an idle row has no background");
    ui.hover = Some(Target::Session(SessionId::new("b")));
    let (hovered, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(raised(&hovered, beta), "the pointer raises its row too");
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
