//! The overview tab: what it names, what it counts, and what it offers.

mod children;
mod delivery;
mod header;
mod markdown;

use groove_types::{Day, ExternalId, SessionId, SessionKind};

use crate::input::{Delta, Input, handle};
use crate::tests::{app, full_app, metrics, render, task, texts_of};
use crate::{Ui, view};

/// The ui on the Overview mode.
fn overviewed() -> Ui {
    let mut ui = Ui::default();
    ui.session.mode = crate::views::session::Mode::Overview;
    ui
}

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
fn a_body_too_tall_for_the_tab_scrolls_and_never_reaches_past_its_width() {
    let mut app = working_a_task();
    let long = "Close the gates and every one of the paths behind them. ".repeat(80);
    app.task.bodies.insert("gh-haoov-groove-50".into(), long);
    let window = metrics(1280, 800, 1.0);
    let mut ui = overviewed();
    let (frame, hits) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let workspace = crate::layout::Layout::of(window, &ui).agent;
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
    let (texts, _) = render(&app, &overviewed(), metrics(1280, 800, 1.0));
    assert!(!texts.iter().any(|t| t == "PROPERTIES"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "BODY"), "{texts:?}");
}

#[test]
fn a_property_the_task_lacks_is_left_out() {
    let mut app = working_a_task();
    let one = app.task.tasks.first_mut().expect("the task");
    one.logged = None;
    one.dates.start = None;
    let (texts, _) = render(&app, &overviewed(), metrics(1280, 800, 1.0));
    assert!(texts.iter().any(|t| t == "due 2026-09-30"), "{texts:?}");
    assert!(
        !texts.iter().any(|t| t == "HOURS"),
        "an estimate alone shows no hours: {texts:?}"
    );
    assert!(!texts.iter().any(|t| t == "—"), "no dash: {texts:?}");
}

#[test]
fn a_worktrees_counts_show_as_icons_and_zeros_do_not() {
    let mut app = full_app();
    let open = app.session.get_mut(&SessionId::new("a")).unwrap();
    let worktree = open.worktrees[0].id.clone();
    open.status.insert(
        worktree,
        groove_types::WorktreeStatus {
            modified: 3,
            staged: 0,
            ahead: 1,
            behind: 0,
        },
    );
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
        &app,
        &overviewed(),
        metrics,
        &mut groove_gfx::Fonts::embedded(),
    );
    let workspace = crate::layout::Layout::of(metrics, &overviewed()).agent;
    let icons: Vec<groove_gfx::Icon> = frame.layers()[0]
        .icons
        .iter()
        .filter(|i| workspace.contains(i.rect.x, i.rect.y))
        .map(|i| i.icon)
        .collect();
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
    let texts: Vec<String> = texts_of(&frame);
    assert!(texts.iter().any(|t| t == "3"));
    assert!(texts.iter().any(|t| t == "1"));
}

#[test]
fn a_session_without_a_worktree_says_so() {
    let mut app = app();
    app.session.selected = Some(SessionId::new("a"));
    let metrics = metrics(1280, 800, 1.0);
    let (texts, _) = render(&app, &overviewed(), metrics);
    assert!(
        texts.iter().any(|t| t == "no repo · no worktree"),
        "{texts:?}"
    );
}

#[test]
fn hours_show_what_was_logged_alone_and_nothing_offers_to_log_more() {
    let mut app = working_a_task();
    let id = ExternalId::new("github.com/haoov/groove#50");
    app.task.time.insert(
        id,
        groove_types::TimeSummary {
            tracked_seconds: 5400,
            logged_seconds: 1800,
            today_seconds: 5400,
            unlogged_seconds: 3600,
        },
    );
    let (texts, _) = render(&app, &overviewed(), metrics(1280, 800, 1.0));
    assert!(!texts.iter().any(|t| t.starts_with("log ")), "{texts:?}");
    assert!(!texts.iter().any(|t| t.contains(" of ")), "{texts:?}");
}

#[test]
fn a_long_repo_name_is_whole_and_stops_before_its_path() {
    let mut app = full_app();
    let long = "a-service-with-a-very-long-name-indeed-for-one-repository";
    if let Some(repo) = app
        .session
        .get_mut(&SessionId::new("a"))
        .and_then(|open| open.repos.first_mut())
    {
        repo.project = long.into();
    }
    let (frame, _) = view(
        &app,
        &overviewed(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let mut fonts = groove_gfx::Fonts::embedded();
    let path = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text.starts_with("gitlab.example.com"))
        .expect("the repo's path");
    let name = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text.starts_with("a-service") && run.y == path.y)
        .expect("the repo's name, on the same row");
    let style = name.style;
    let ends = name.x + fonts.measure(&name.text, style.font, style.weight, style.size);
    assert!(
        ends <= path.x,
        "the name ends before the path starts: {ends} against {}",
        path.x
    );
    assert_eq!(name.text, long, "and is never cut for it");
}
