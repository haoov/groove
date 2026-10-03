//! The sidebar's commits: what the list says, what a click shows, and the way back.

use groove_types::{CommitEntry, Timestamp};

use super::*;
use crate::tests::sidebar_ui;
use crate::views::session::Pane;

fn commit(sha: &str, message: &str, is_base: bool) -> CommitEntry {
    CommitEntry {
        sha: format!("{sha}0000000000000000000000000000000"),
        short_sha: sha.to_string(),
        message: message.to_string(),
        author: "T".into(),
        at: Timestamp::new(0),
        is_base,
    }
}

/// The fixture with those commits and the commits list up.
fn logged(log: Vec<CommitEntry>) -> (AppState, Ui) {
    let mut app = with_files(&["src/lib.rs"]);
    app.workspace.log = log;
    let mut ui = sidebar_ui();
    ui.session.pane = Pane::Commits;
    (app, ui)
}

fn in_sidebar(app: &AppState, ui: &Ui) -> Vec<String> {
    let (frame, _) = view(app, ui, window(), &mut Fonts::embedded());
    let rect = Layout::of(window(), ui).sidebar;
    frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.x >= rect.x && run.x < rect.right())
        .map(|run| run.text.clone())
        .collect()
}

#[test]
fn the_list_says_each_commit_its_name_and_who_made_it() {
    let (app, ui) = logged(vec![commit("abc1234", "feat: one", false)]);
    let drawn = in_sidebar(&app, &ui);
    assert!(drawn.iter().any(|one| one == "abc1234"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "feat: one"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "T"), "{drawn:?}");
}

#[test]
fn the_strip_counts_the_commits_the_branch_owns() {
    let (app, ui) = logged(vec![
        commit("abc1234", "feat: one", false),
        commit("def5678", "chore: base", true),
    ]);
    let drawn = in_sidebar(&app, &ui);
    assert!(
        drawn.iter().any(|one| one == "commits · 1"),
        "the base's own are not the branch's: {drawn:?}"
    );
}

#[test]
fn the_list_says_when_it_holds_nothing_yet() {
    let (app, ui) = logged(Vec::new());
    let drawn = in_sidebar(&app, &ui);
    assert!(
        drawn.iter().any(|one| one == "reading the commits…"),
        "{drawn:?}"
    );
}

#[test]
fn picking_the_commits_list_reads_them() {
    let app = with_files(&["src/lib.rs"]);
    let mut ui = sidebar_ui();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let strip = hits
        .rect_of(&Target::Pane(Pane::Commits))
        .expect("the strip names them");
    click(strip, &mut ui, &app, &hits);
    assert_eq!(ui.session.pane, Pane::Commits);
    let asked = crate::render::layout_commands(&app, &ui, window());
    assert!(
        asked.iter().any(|one| matches!(
            one,
            groove_controllers::Command::Workspace(
                groove_controllers::workspace::Command::GetCommits
            )
        )),
        "the frame asks for them: {asked:?}"
    );
}

#[test]
fn a_click_on_a_commit_asks_for_its_change() {
    let one = commit("abc1234", "feat: one", false);
    let (app, mut ui) = logged(vec![one.clone()]);
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&Target::Commit(one.sha.clone()))
        .expect("the commit's row");
    let commands = click(row, &mut ui, &app, &hits);
    assert_eq!(
        commands,
        [groove_controllers::Command::Workspace(
            groove_controllers::workspace::Command::OpenCommit { sha: one.sha }
        )]
    );
}

#[test]
fn a_shown_commit_heads_every_list_with_the_way_back() {
    let one = commit("abc1234", "feat: one", false);
    let (mut app, mut ui) = logged(vec![one.clone()]);
    app.workspace.commit = Some(one);
    for pane in Pane::ALL {
        ui.session.pane = pane;
        let drawn = in_sidebar(&app, &ui);
        let head = ["showing commit", "abc1234", "working tree"];
        assert!(
            head.iter()
                .all(|said| drawn.iter().any(|text| text == said)),
            "{pane:?}: {drawn:?}"
        );
        assert!(
            !drawn
                .iter()
                .any(|text| text == "feat: one" && pane != Pane::Commits),
            "{drawn:?}"
        );
    }

    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let back = hits.rect_of(&Target::Working).expect("the way back");
    let commands = click(back, &mut ui, &app, &hits);
    assert_eq!(
        commands,
        [groove_controllers::Command::Workspace(
            groove_controllers::workspace::Command::LeaveCommit
        )]
    );
}

#[test]
fn no_commit_box_and_no_mode_stand_while_a_commit_is_shown() {
    let one = commit("abc1234", "feat: one", false);
    let mut app = with_files(&["src/lib.rs"]);
    app.workspace.commit = Some(one);
    let mut ui = sidebar_ui();
    ui.settle(&app);
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    for gone in [
        Target::Message,
        Target::Do,
        Target::Mode(groove_types::DiffMode::Working),
    ] {
        assert!(hits.rect_of(&gone).is_none(), "{gone:?} is not offered");
    }
    assert!(
        hits.rect_of(&Target::File("src/lib.rs".into())).is_some(),
        "the files it changed"
    );
}
