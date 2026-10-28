//! The strip's header and its info line: the title, the tabs, the pickers, the task's own actions.

use groove_types::SessionId;

use super::working_a_task;
use crate::tests::{full_app, metrics, texts_of};
use crate::{Ui, view};

#[test]
fn the_header_names_the_session_and_what_it_points_at() {
    let app = full_app();
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics,
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts: Vec<String> = texts_of(&frame);
    assert!(texts.iter().any(|t| t == "Alpha"), "the title");
    assert!(texts.iter().any(|t| t == "kiwi"), "the repo picker");
    assert_eq!(
        texts.iter().filter(|t| *t == "explorer/alpha").count(),
        2,
        "the worktree picker and the overview row"
    );
    let info = crate::layout::Layout::of(metrics, &Ui::default()).info;
    let carets = frame.layers()[0]
        .icons
        .iter()
        .filter(|i| i.icon == groove_gfx::Icon::CaretDown)
        .filter(|i| info.contains(i.rect.x, i.rect.y))
        .count();
    assert_eq!(carets, 2, "the repo and worktree pickers");
}

#[test]
fn a_long_title_is_cut_so_the_pickers_stay_inside_the_header() {
    let mut app = full_app();
    app.session
        .get_mut(&SessionId::new("a"))
        .expect("the fixture's session")
        .session
        .title = "Harden Groove: CI gates, security fixes, defect fixes, typed errors".into();
    let window = metrics(1280, 800, 1.0);
    let ui = Ui::default();
    let (frame, hits) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let header = crate::layout::Layout::of(window, &ui).info;
    for picker in [crate::hit::Picks::Repo, crate::hit::Picks::Branch] {
        let box_ = hits
            .rect_of(&crate::hit::Target::Picker(picker))
            .expect("the header holds both pickers");
        assert!(
            box_.right() <= header.right(),
            "{picker:?} at {box_:?} runs past {header:?}"
        );
    }
    let cut = frame.layers()[0]
        .texts
        .iter()
        .any(|t| t.text.starts_with("Harden Groove") && t.text.ends_with('\u{2026}'));
    assert!(cut, "the title carries the ellipsis");
}

#[test]
fn the_header_holds_the_title_over_the_info_line_s_pickers() {
    let app = full_app();
    let window = metrics(1280, 800, 1.0);
    let ui = Ui::default();
    let (frame, hits) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let layout = crate::layout::Layout::of(window, &ui);
    let title = frame.layers()[0]
        .texts
        .iter()
        .find(|t| t.text == "Alpha" && t.x >= layout.header.x)
        .expect("the header's title")
        .y;
    let box_ = hits
        .rect_of(&crate::hit::Target::Picker(crate::hit::Picks::Repo))
        .expect("the repo picker");
    assert!(
        title < layout.header.bottom(),
        "the title is on the first line"
    );
    assert!(box_.y >= layout.info.y, "the pickers are on the second");
    assert!(box_.bottom() <= layout.info.bottom());
    assert_eq!(
        layout.workspace.y,
        layout.info.bottom(),
        "the tab starts under both lines"
    );
    for which in [crate::hit::Picks::Repo, crate::hit::Picks::Branch] {
        let box_ = hits
            .rect_of(&crate::hit::Target::Picker(which))
            .expect("a picker");
        assert!(
            frame.layers()[0].quads.iter().all(|quad| quad.rect != box_),
            "{which:?} lies flat on the glass"
        );
    }
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
    let mut ui = Ui::default();
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
