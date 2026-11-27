//! The strip's header and its info line: the title, the tabs, the worktree, the task's own actions.

use groove_types::SessionId;

use super::working_a_task;
use crate::tests::{full_app, metrics};
use crate::view;

#[test]
fn a_long_title_is_cut_so_the_worktree_stays_inside_the_header() {
    let mut app = full_app();
    app.session
        .get_mut(&SessionId::new("a"))
        .expect("the fixture's session")
        .session
        .title = "Harden Groove: CI gates, security fixes, defect fixes, typed errors".into();
    let window = metrics(1280, 800, 1.0);
    let ui = super::overviewed();
    let (frame, _) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let header = crate::layout::Layout::of(window, &ui).top;
    let run = info_run(&frame, header, "kiwi");
    assert!(run.x < header.right(), "past {header:?}");
    let cut = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .any(|t| t.text.starts_with("Harden Groove") && t.text.ends_with('\u{2026}'));
    assert!(cut, "the title carries the ellipsis");
}

/// The text run starting with `label` inside `info`.
fn info_run(frame: &groove_gfx::Frame, info: groove_gfx::Rect, label: &str) -> groove_gfx::TextRun {
    let runs = frame.layers().iter().flat_map(|layer| layer.texts.iter());
    let mut on =
        runs.filter(|run| run.text.starts_with(label) && info.contains(run.x, run.y + 1.0));
    on.next()
        .cloned()
        .unwrap_or_else(|| panic!("{label} inside {info:?}"))
}

/// What the palette's row to delete the session here sends, if it offers one.
fn deleted_here(app: &groove_controllers::AppState) -> Option<groove_controllers::Command> {
    crate::palette::entries(app)
        .into_iter()
        .find(|entry| entry.label == "Delete the session locally")
        .and_then(|entry| match entry.run {
            crate::palette::Run::Command(command) => Some(command),
            _ => None,
        })
}

#[test]
fn a_task_or_a_review_is_deleted_here_from_the_palette_and_an_explorer_is_not() {
    let session = SessionId::new("a");
    let away =
        groove_controllers::Command::Session(groove_controllers::session::Command::DeleteLocal {
            session,
        });
    assert_eq!(deleted_here(&working_a_task()), Some(away.clone()));
    assert_eq!(deleted_here(&reviewing()), Some(away));
    assert_eq!(deleted_here(&crate::tests::full_app()), None);
}

/// The fixture's session, reviewing someone else's merge request.
fn reviewing() -> groove_controllers::AppState {
    let mut app = crate::tests::full_app();
    app.session
        .get_mut(&SessionId::new("a"))
        .expect("the fixture's session")
        .session
        .kind = groove_types::SessionKind::Review {
        project: "g/kiwi".into(),
        iid: 7,
    };
    app
}

#[test]
fn a_task_s_title_is_followed_by_what_opens_its_page() {
    let mut app = working_a_task();
    let url = "https://github.com/haoov/groove/issues/50".to_string();
    app.task.tasks[0].url = Some(url.clone());
    let mut ui = super::overviewed();
    let window = metrics(1280, 800, 1.0);
    let (frame, hits) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let page = crate::hit::Target::TaskPage(url.clone());
    let button = hits.rect_of(&page).expect("the button");
    let title = frame.layers()[0]
        .texts
        .iter()
        .find(|t| t.text == "Alpha")
        .expect("the title");
    assert!(button.x > title.x, "after the title");

    let press = crate::input::Input::Press {
        x: button.x + button.w / 2.0,
        y: button.y + button.h / 2.0,
        mods: Default::default(),
    };
    let commands = crate::input::handle(press, &mut ui, &app, &hits, window);
    let browse = groove_controllers::task::Command::Browse { url };
    assert_eq!(commands, [groove_controllers::Command::Task(browse)]);
}
