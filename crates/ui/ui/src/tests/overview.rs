//! The overview tab: what it names, what it counts, and what it offers.

mod header;

use groove_types::{Day, ExternalId, SessionId, SessionKind};

use crate::input::{Delta, Input, handle};
use crate::tests::{app, full_app, metrics, task};
use crate::{Ui, view};

/// A session that works a task, with the task and its body read.
fn working_a_task() -> groove_controllers::AppState {
    let mut app = full_app();
    let external = ExternalId::new("github.com/haoov/groove#50");
    let mut one = task("gh-haoov-groove-50", "Harden Groove", external.as_str());
    one.dates.start = Some(Day::parse("2026-09-14").unwrap());
    one.dates.due = Some(Day::parse("2026-09-30").unwrap());
    app.task.tasks = vec![one];
    app.task.bodies.insert(
        "gh-haoov-groove-50".into(),
        "Close the gates before the release.".into(),
    );
    app.session
        .get_mut(&SessionId::new("a"))
        .expect("the fixture's session")
        .session
        .kind = SessionKind::Task {
        external_id: external,
    };
    app
}

#[test]
fn a_task_session_shows_the_six_properties_and_the_body() {
    let app = working_a_task();
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    for named in [
        "PROPERTIES",
        "Status",
        "Priority",
        "Start",
        "Due",
        "Estimate",
        "Logged",
    ] {
        assert!(texts.iter().any(|t| t == named), "{named}: {texts:?}");
    }
    assert!(texts.iter().any(|t| t == "In progress"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "2026-09-30"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "4h"), "the estimate: {texts:?}");
    assert!(texts.iter().any(|t| t == "1.5h"), "the hours: {texts:?}");
    assert!(texts.iter().any(|t| t == "BODY"), "{texts:?}");
    assert!(
        texts
            .iter()
            .any(|t| t == "Close the gates before the release."),
        "{texts:?}"
    );
}

#[test]
fn a_body_too_tall_for_the_tab_scrolls_and_never_reaches_past_its_width() {
    let mut app = working_a_task();
    let long = "Close the gates and every one of the paths behind them. ".repeat(80);
    app.task.bodies.insert("gh-haoov-groove-50".into(), long);
    let window = metrics(1280, 800, 1.0);
    let mut ui = Ui::default();
    let (frame, hits) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let workspace = crate::layout::Layout::of(window, &ui).workspace;
    assert!(
        hits.extent(crate::hit::Scroller::Overview) > 0.0,
        "the body is taller than the tab"
    );
    let over = frame.layers()[0]
        .texts
        .iter()
        .filter(|t| t.x > workspace.right())
        .count();
    assert_eq!(over, 0, "every line is wrapped inside the tab");

    let first = frame.layers()[0]
        .texts
        .iter()
        .find(|t| t.text.starts_with("Close the gates"))
        .map(|t| t.y)
        .expect("the body's first line");
    handle(
        Input::Scroll {
            x: workspace.x + 10.0,
            y: workspace.y + 10.0,
            delta: Delta::Pixels {
                across: 0.0,
                down: -120.0,
            },
        },
        &mut ui,
        &app,
        &hits,
        window,
    );
    assert!(ui.session.overview > 0.0, "the wheel moved the tab");
    let (scrolled, _) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let now = scrolled.layers()[0]
        .texts
        .iter()
        .find(|t| t.text.starts_with("Close the gates"))
        .map(|t| t.y);
    assert!(
        now.is_none_or(|y| y < first),
        "the body moved up: {now:?} was {first}"
    );
}

#[test]
fn an_explorer_shows_no_properties_and_no_body() {
    let app = full_app();
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(!texts.iter().any(|t| t == "PROPERTIES"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "BODY"), "{texts:?}");
}

#[test]
fn a_property_the_task_has_no_value_for_reads_as_a_dash() {
    let mut app = working_a_task();
    let one = app.task.tasks.first_mut().expect("the task");
    one.logged = None;
    one.dates.start = None;
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let dashes = frame.layers()[0]
        .texts
        .iter()
        .filter(|t| t.text == "—")
        .count();
    assert_eq!(dashes, 2, "the start and the hours");
}

#[test]
fn the_overview_lists_repos_and_worktrees_with_the_selected_one_marked() {
    let app = full_app();
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics,
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "overview"));
    assert!(texts.iter().any(|t| t == "REPOS AND WORKTREES"));
    assert!(texts.iter().any(|t| t == "mayo"));
    assert_eq!(
        texts.iter().filter(|t| *t == "explorer/alpha").count(),
        2,
        "the header and the row"
    );
    let icons: Vec<groove_gfx::Icon> = frame.layers()[0].icons.iter().map(|i| i.icon).collect();
    assert!(
        icons.contains(&groove_gfx::Icon::Cube),
        "the repo wears a mark"
    );
    assert!(
        icons.contains(&groove_gfx::Icon::Compass),
        "the session's kind"
    );
}

#[test]
fn a_worktrees_counts_show_as_icons_and_zeros_do_not() {
    let mut app = full_app();
    let open = app.session.get_mut(&SessionId::new("a")).unwrap();
    let worktree = open.worktrees[0].id.clone();
    open.delivery.push((
        worktree,
        groove_types::WorktreeDelivery {
            status: groove_types::WorktreeStatus {
                modified: 3,
                staged: 0,
                ahead: 1,
                behind: 0,
            },
            ..Default::default()
        },
    ));
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics,
        &mut groove_gfx::Fonts::embedded(),
    );
    let icons: Vec<groove_gfx::Icon> = frame.layers()[0].icons.iter().map(|i| i.icon).collect();
    assert!(icons.contains(&groove_gfx::Icon::ArrowUp), "ahead 1");
    assert!(icons.contains(&groove_gfx::Icon::Dot), "modified 3");
    assert!(
        !icons.contains(&groove_gfx::Icon::Plus),
        "staged 0 is not drawn"
    );
    assert!(
        !icons.contains(&groove_gfx::Icon::ArrowDown),
        "behind 0 is not drawn"
    );
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "3"));
    assert!(texts.iter().any(|t| t == "1"));
}

#[test]
fn a_session_without_a_worktree_says_so() {
    let mut app = app();
    app.session.selected = Some(SessionId::new("a"));
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics,
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "no repo"));
    assert!(texts.iter().any(|t| t == "no worktree"));
}

#[test]
fn a_rule_stands_between_the_overview_s_sections() {
    let app = working_a_task();
    let window = metrics(1280, 800, 1.0);
    let ui = Ui::default();
    let (frame, _) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let tokens = crate::tokens::Tokens::new(1.0);
    let styles = crate::style::Styles::new(app.config.theme(), tokens);
    let workspace = crate::layout::Layout::of(window, &ui).workspace;
    let wide = workspace.w - tokens.md * 2.0;
    let rules = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.line() && quad.rect.h == tokens.hairline)
        .filter(|quad| quad.rect.x == workspace.x + tokens.md && quad.rect.w == wide)
        .count();
    assert_eq!(rules, 2, "one above the repos, one above the body");
}

#[test]
fn the_hours_the_clock_measured_are_offered_to_the_source() {
    let mut app = working_a_task();
    let id = ExternalId::new("github.com/haoov/groove#50");
    app.task.time.insert(
        id.clone(),
        groove_types::TimeSummary {
            tracked_seconds: 5400,
            logged_seconds: 1800,
            today_seconds: 5400,
            unlogged_seconds: 3600,
        },
    );
    let window = metrics(1280, 800, 1.0);
    let mut ui = Ui::default();
    let (frame, hits) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "log 1h"), "{texts:?}");
    let button = hits
        .rect_of(&crate::hit::Target::LogHours(id.clone()))
        .expect("the hours are offered");
    let commands = crate::tests::click(button, &mut ui, &app, &hits);
    assert_eq!(
        commands,
        [groove_controllers::Command::Task(
            groove_controllers::task::Command::LogHours { external_id: id }
        )]
    );
}

#[test]
fn a_task_with_nothing_measured_offers_no_hours() {
    let app = working_a_task();
    let (_, hits) = view(
        &app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let id = ExternalId::new("github.com/haoov/groove#50");
    assert!(hits.rect_of(&crate::hit::Target::LogHours(id)).is_none());
}
