//! The review column: what it names of each MR, and what the filter leaves.

use groove_types::{Forge, ReviewMr, Timestamp};

use super::*;

fn asked(project: &str, iid: u64, title: &str, author: &str, ago: i64) -> ReviewMr {
    ReviewMr {
        forge: Forge::Github,
        project: project.into(),
        iid,
        title: title.into(),
        author: author.into(),
        source_branch: "fix/one".into(),
        target_branch: "main".into(),
        draft: false,
        web_url: format!("https://github.com/{project}/pull/{iid}"),
        updated_at: Timestamp::new(-ago),
        local_path: None,
        approved: false,
    }
}

fn waiting() -> AppState {
    let mut app = full_app();
    app.delivery.reviews = vec![
        asked(
            "acme/groove",
            7,
            "fix(forge): read the checks",
            "someone",
            3600,
        ),
        asked("acme/other", 12, "feat: a thing", "another", 7200),
    ];
    app
}

fn texts(app: &AppState, ui: &Ui) -> Vec<String> {
    let (frame, _) = view(app, ui, window(), &mut Fonts::embedded());
    frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .map(|one| one.text.clone())
        .collect()
}

#[test]
fn each_mr_names_its_project_its_number_its_author_and_its_age() {
    let app = waiting();
    let (ui, _) = on_board(&app);
    let drawn = texts(&app, &ui);
    assert!(
        drawn.iter().any(|t| t == "fix(forge): read the checks"),
        "{drawn:?}"
    );
    assert!(
        drawn.iter().any(|t| t == "acme/groove#7 · someone"),
        "the project, the number and who wrote it: {drawn:?}"
    );
    assert!(drawn.iter().any(|t| t == "1h"), "how long ago: {drawn:?}");
    assert!(
        drawn.iter().any(|t| t == "REVIEW · 2"),
        "the column counts them: {drawn:?}"
    );
}

#[test]
fn an_mr_of_the_column_can_be_pointed_at() {
    let app = waiting();
    let (_, hits) = on_board(&app);
    assert!(
        hits.rect_of(&Target::Review("acme/groove".into(), 7))
            .is_some(),
        "the row is there to be opened"
    );
}

#[test]
fn nothing_waiting_says_so() {
    let app = full_app();
    let (ui, _) = on_board(&app);
    let drawn = texts(&app, &ui);
    assert!(
        drawn.iter().any(|t| t == "nothing is waiting on you"),
        "{drawn:?}"
    );
}

#[test]
fn the_filter_narrows_the_column_by_title_author_and_repo() {
    let app = waiting();
    let mut ui = on_board(&app).0;
    ui.board.filter.set("another");
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "feat: a thing"), "{drawn:?}");
    assert!(
        !drawn.iter().any(|t| t == "fix(forge): read the checks"),
        "the other author's is gone: {drawn:?}"
    );

    ui.board.filter.set("repo:other");
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "feat: a thing"), "{drawn:?}");

    ui.board.filter.set("kind:review");
    let drawn = texts(&app, &ui);
    assert!(
        drawn.iter().any(|t| t == "feat: a thing"),
        "both stay, and the other columns empty: {drawn:?}"
    );
}

#[test]
fn clicking_an_mr_opens_the_session_that_reviews_it() {
    let app = waiting();
    let (mut ui, hits) = on_board(&app);
    let target = Target::Review("acme/groove".into(), 7);
    let rect = hits.rect_of(&target).expect("the row was drawn");
    let asked = crate::input::handle(
        crate::input::Input::Press {
            x: rect.x + rect.w / 2.0,
            y: rect.y + rect.h / 2.0,
            mods: Default::default(),
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert_eq!(
        asked,
        vec![groove_controllers::Command::Session(
            groove_controllers::session::Command::OpenReview {
                project: "acme/groove".into(),
                iid: 7,
            }
        )]
    );
}
