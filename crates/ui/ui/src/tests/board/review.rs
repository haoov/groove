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
        review: None,
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

#[test]
fn a_review_row_says_where_its_reviewers_stand() {
    let mut app = waiting();
    app.delivery.reviews[0].review = Some(groove_types::ReviewState::Commented);
    let (ui, _) = on_board(&app);
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "comments"), "{drawn:?}");
}

/// The titles the column drew, top to bottom.
fn order(app: &AppState, ui: &Ui) -> Vec<String> {
    let (frame, _) = view(app, ui, window(), &mut Fonts::embedded());
    let mut titles: Vec<(f32, String)> = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .filter(|one| app.delivery.reviews.iter().any(|mr| mr.title == one.text))
        .map(|one| (one.y, one.text.clone()))
        .collect();
    titles.sort_by(|one, two| one.0.total_cmp(&two.0));
    titles.into_iter().map(|(_, title)| title).collect()
}

fn key(key: Key, ui: &mut Ui, app: &AppState) -> Vec<groove_controllers::Command> {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let input = crate::input::Input::Key {
        key,
        mods: Default::default(),
    };
    crate::input::handle(input, ui, app, &hits, window())
}

fn many(count: u64) -> AppState {
    let mut app = full_app();
    app.delivery.reviews = (0..count)
        .map(|at| asked("acme/many", at, &format!("mr {at}"), "someone", at as i64))
        .collect();
    app
}

#[test]
fn a_review_row_s_title_stands_in_its_first_line() {
    let app = waiting();
    let (ui, hits) = on_board(&app);
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&Target::Review("acme/groove".into(), 7))
        .expect("the row");
    let texts = &frame.layers()[0].texts;
    let at = |text: &str| texts.iter().find(|t| t.text == text).map(|t| t.y);
    let title = at("fix(forge): read the checks").expect("the title");
    let under = at("acme/groove#7 · someone").expect("the line under it");
    let tokens = groove_ui_kit::base::tokens::Tokens::new(1.0);
    let first = tokens.row + tokens.sm;
    assert!(
        title >= row.y && title < row.y + first,
        "{title} in {row:?}"
    );
    assert!(
        under >= row.y + first,
        "{under} under the first line of {row:?}"
    );
}

#[test]
fn review_rows_stand_apart_with_a_rule_under_each() {
    let app = waiting();
    let (ui, hits) = on_board(&app);
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = groove_ui_kit::base::style::Styles::new(
        app.config.theme(),
        groove_ui_kit::base::tokens::Tokens::new(1.0),
    );
    let row = hits
        .rect_of(&Target::Review("acme/groove".into(), 7))
        .expect("the row");
    let ruled = frame.layers()[0]
        .quads
        .iter()
        .any(|quad| quad.color == styles.line() && (quad.rect.bottom() - row.bottom()).abs() < 1.0);
    assert!(ruled, "a rule along the row's foot");
}

#[test]
fn the_column_shows_the_last_updated_first() {
    let app = waiting();
    let (ui, _) = on_board(&app);
    assert_eq!(
        order(&app, &ui),
        vec!["fix(forge): read the checks", "feat: a thing"]
    );
}

#[test]
fn clicking_a_heading_orders_the_column_by_it_and_again_turns_it_round() {
    let app = waiting();
    let (mut ui, hits) = on_board(&app);
    let heading = Target::SortReview(crate::views::board::review::By::Title);
    let rect = hits.rect_of(&heading).expect("the heading");
    click(rect, &mut ui, &app, &hits);
    assert_eq!(
        order(&app, &ui),
        vec!["feat: a thing", "fix(forge): read the checks"]
    );
    click(rect, &mut ui, &app, &hits);
    assert_eq!(
        order(&app, &ui),
        vec!["fix(forge): read the checks", "feat: a thing"]
    );
}

#[test]
fn down_selects_the_first_review_and_enter_opens_it() {
    let app = waiting();
    let (mut ui, _) = on_board(&app);
    key(Key::Down, &mut ui, &app);
    assert_eq!(ui.board.chosen, Some(("acme/groove".into(), 7)));
    let asked = key(Key::Enter, &mut ui, &app);
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

#[test]
fn the_selection_stays_on_its_mr_when_the_order_changes() {
    let app = waiting();
    let (mut ui, _) = on_board(&app);
    key(Key::Down, &mut ui, &app);
    ui.board.reviews = ui
        .board
        .reviews
        .clicked(crate::views::board::review::By::Title);
    key(Key::Up, &mut ui, &app);
    assert_eq!(ui.board.chosen, Some(("acme/other".into(), 12)));
}

#[test]
fn escape_lets_the_selection_go() {
    let app = waiting();
    let (mut ui, _) = on_board(&app);
    key(Key::Down, &mut ui, &app);
    key(Key::Escape, &mut ui, &app);
    assert_eq!(ui.board.chosen, None);
}

#[test]
fn only_the_rows_on_screen_are_drawn() {
    let app = many(2000);
    let (ui, _) = on_board(&app);
    let drawn = order(&app, &ui).len();
    assert!(drawn > 0 && drawn < 100, "{drawn} rows drawn of 2000");
}

#[test]
fn the_selection_scrolls_into_view_past_the_bottom() {
    let app = many(200);
    let (mut ui, _) = on_board(&app);
    for _ in 0..150 {
        key(Key::Down, &mut ui, &app);
    }
    let chosen = ui.board.chosen.clone().expect("a selection");
    let title = format!("mr {}", chosen.1);
    assert!(ui.board.review > 0.0, "the column scrolled");
    assert!(order(&app, &ui).contains(&title), "{title} is on screen");
}
