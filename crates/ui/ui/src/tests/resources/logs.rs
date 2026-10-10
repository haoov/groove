//! A pod's logs: the stream its view asks for, its lines drawn by level, and following them.

use groove_controllers::cluster::Command as Cluster;
use groove_controllers::cluster_service::Logged;
use groove_controllers::{AppState, Command};
use groove_types::{LogKey, LogLine, LogRange, LogSource};

use super::pods;
use super::tab::{crashing, lands, link, opened, runs, wheel};
use crate::Ui;
use crate::hit::Target;
use crate::tests::{click, window};
use crate::views::session::resources::View;

fn worker(previous: bool, range: LogRange) -> LogKey {
    LogKey {
        context: "staging".into(),
        namespace: "paxone".into(),
        pod: "api-0".into(),
        source: LogSource::Container("worker".into()),
        previous,
        range,
    }
}

/// The crashing pod read, its tab on its logs view.
fn on_logs() -> (AppState, Ui) {
    let (mut app, mut ui) = opened();
    lands(&mut app, &link("api-0", pods()).key(), vec![crashing()]);
    hit(&app, &mut ui, Target::ResourceView(View::Logs));
    (app, ui)
}

/// A click on what was drawn for `target`.
fn hit(app: &AppState, ui: &mut Ui, target: Target) -> Vec<Command> {
    let hits = seen(app, ui);
    let at = hits.rect_of(&target);
    let at = at.unwrap_or_else(|| panic!("{target:?} drawn"));
    click(at, ui, app, &hits)
}

fn seen(app: &AppState, ui: &Ui) -> crate::Hits {
    let metrics = crate::tests::metrics(1920, 1080, 1.0);
    crate::view(app, ui, metrics, &mut groove_gfx::Fonts::embedded()).1
}

fn asks(app: &AppState, ui: &Ui, key: LogKey) -> bool {
    let follow = Cluster::FollowLogs {
        reader: "resource".into(),
        key: Box::new(key),
    };
    crate::frame_commands(app, ui, window()).contains(&Command::Cluster(follow))
}

#[test]
fn the_logs_view_asks_for_the_pod_s_first_container_and_its_previous_run_on_a_click() {
    let (app, mut ui) = on_logs();
    assert!(asks(&app, &ui, worker(false, LogRange::Since(900))));
    let texts: Vec<String> = runs(&app, &ui).into_iter().map(|one| one.text).collect();
    assert!(
        texts.iter().any(|one| one == "opening the logs…"),
        "{texts:?}"
    );
    hit(&app, &mut ui, Target::LogPrevious(true));
    assert!(asks(&app, &ui, worker(true, LogRange::Since(900))));
}

#[test]
fn lines_draw_by_level_follow_the_end_pause_under_the_wheel_and_find_through_the_editor() {
    let (mut app, mut ui) = on_logs();
    let key = worker(false, LogRange::Since(900));
    app.cluster.store.logs.lease(&key, "resource");
    let mut lines: Vec<LogLine> = (0..199)
        .map(|at| LogLine::read(&format!("2026-10-09T14:02:09.5Z INFO job {at} done"), None))
        .collect();
    lines.push(LogLine::read(
        "2026-10-09T14:02:10Z ERROR db unreachable",
        None,
    ));
    app.cluster.store.logs.apply(&key, Logged::Lines(lines));
    let drawn = runs(&app, &ui);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), window().tokens());
    let error = drawn
        .iter()
        .find(|one| one.text.contains("ERROR db unreachable"));
    let red = styles.syntax(groove_types::Capture::Error);
    assert_eq!(
        error.map(|one| one.style.color),
        Some(red),
        "the last line, in red"
    );
    let rows = seen(&app, &ui).rect_of(&Target::Code).expect("the lines");
    wheel(&app, &mut ui, (rows.x + 40.0, rows.y + 40.0), (0.0, 120.0));
    let tab = ui.session.resources.tab().expect("the tab");
    assert!(!tab.logs.following, "the wheel up pauses");
    hit(&app, &mut ui, Target::LogFollow);
    assert!(
        ui.session
            .resources
            .tab()
            .is_some_and(|one| one.logs.following)
    );
    let ctrl = crate::input::Modifiers {
        ctrl: true,
        ..Default::default()
    };
    crate::tests::press(crate::input::Key::Char('f'), ctrl, &mut ui, &app);
    let mut asked = Vec::new();
    for c in "unreachable".chars() {
        asked = crate::tests::press(
            crate::input::Key::Char(c),
            Default::default(),
            &mut ui,
            &app,
        );
    }
    let found = ui.session.find.as_ref().map_or(0, |find| find.hits.len());
    assert_eq!(found, 1);
    let caret = |one: &Command| matches!(one, Command::Cluster(Cluster::LogCaret { .. }));
    assert!(asked.iter().any(caret), "{asked:?}");
}

#[test]
fn a_range_picked_opens_the_stream_again_from_there_and_code_shrinks_while_logs_show() {
    let (app, mut ui) = on_logs();
    hit(&app, &mut ui, Target::LogRange);
    hit(&app, &mut ui, Target::MenuRow(3));
    assert!(asks(&app, &ui, worker(false, LogRange::Since(21_600))));
    let texts: Vec<String> = runs(&app, &ui).into_iter().map(|one| one.text).collect();
    assert!(texts.iter().any(|one| one == "last 6h ▾"), "{texts:?}");
    let small = 13.0 * groove_ui_kit::base::tokens::SMALL;
    assert_eq!(crate::code_size(&app, &ui, (13.0, 14.0)), small);
    hit(&app, &mut ui, Target::ResourceView(View::Yaml));
    assert_eq!(
        crate::code_size(&app, &ui, (13.0, 14.0)),
        14.0,
        "the yaml keeps its size"
    );
}
