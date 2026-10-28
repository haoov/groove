//! The overview tab: what it names, what it counts, and what it offers.

mod delivery;
mod header;
mod markdown;

use groove_types::{Day, ExternalId, SessionId, SessionKind};

use crate::input::{Delta, Input, handle};
use crate::tests::{app, full_app, metrics, render, task, texts_of};
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
fn a_task_session_shows_its_properties_on_one_line_and_the_body() {
    let app = working_a_task();
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts = &frame.layers()[0].texts;
    let at = |said: &str| {
        let one = texts.iter().find(|t| t.text == said);
        one.unwrap_or_else(|| panic!("{said}: {:?}", texts_of(&frame)))
            .y
    };
    let line = at("In progress");
    for said in ["2026-09-14 → 2026-09-30", "1.5h of 4h"] {
        assert_eq!(at(said), line, "{said} on the status's line");
    }
    assert!(at("PROPERTIES") < line, "the heading above");
    assert!(at("Close the gates before the release.") > line);
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
    let (texts, _) = render(&app, &Ui::default(), metrics(1280, 800, 1.0));
    assert!(!texts.iter().any(|t| t == "PROPERTIES"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "BODY"), "{texts:?}");
}

#[test]
fn a_property_the_task_lacks_is_left_out() {
    let mut app = working_a_task();
    let one = app.task.tasks.first_mut().expect("the task");
    one.logged = None;
    one.dates.start = None;
    let (texts, _) = render(&app, &Ui::default(), metrics(1280, 800, 1.0));
    for said in ["due 2026-09-30", "4h estimated"] {
        assert!(texts.iter().any(|t| t == said), "{said}: {texts:?}");
    }
    assert!(!texts.iter().any(|t| t == "—"), "no dash: {texts:?}");
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
    let texts: Vec<String> = texts_of(&frame);
    assert!(texts.iter().any(|t| t == "Overview"));
    assert!(texts.iter().any(|t| t == "REPOS AND WORKTREES"));
    assert!(texts.iter().any(|t| t == "kiwi"));
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
        &Ui::default(),
        metrics,
        &mut groove_gfx::Fonts::embedded(),
    );
    let workspace = crate::layout::Layout::of(metrics, &Ui::default()).workspace;
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
    let (texts, _) = render(&app, &Ui::default(), metrics);
    assert!(texts.iter().any(|t| t == "no repo"));
    assert!(texts.iter().any(|t| t == "no worktree"));
}

#[test]
fn each_section_stands_in_its_own_card_inset_from_the_panel() {
    let app = working_a_task();
    let window = metrics(1280, 800, 1.0);
    let ui = Ui::default();
    let (frame, _) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let tokens = groove_ui_kit::base::tokens::Tokens::new(1.0);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), tokens);
    let workspace = crate::layout::Layout::of(window, &ui).workspace;
    let cards: Vec<_> = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.glass_card() && quad.shape.radius == tokens.glass_round)
        .collect();
    assert_eq!(cards.len(), 3, "the properties, the repos, the body");
    for card in cards {
        assert_eq!(
            card.rect.x,
            workspace.x + tokens.md,
            "a padding at the left"
        );
        assert_eq!(
            card.rect.right(),
            workspace.right() - tokens.md,
            "and at the right"
        );
    }
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
    let (texts, hits) = render(&app, &ui, window);
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
        &Ui::default(),
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
