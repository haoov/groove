//! The Overview tab in the modes layout: each repo apart on its card, the sidebar on the
//! session's kind.

use groove_gfx::Fonts;

use super::moded;
use crate::hit::Target;
use crate::tests::chat::talking;
use crate::tests::{all_texts, window};
use crate::views::session::Mode;

fn overview() -> crate::Ui {
    let mut ui = moded();
    ui.session.mode = Mode::Overview;
    ui
}

#[test]
fn the_overview_shows_each_repo_apart_with_its_worktree_in_a_row() {
    let app = talking(Vec::new());
    let ui = overview();
    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let texts = all_texts(&frame);
    for said in [
        "REPOS · 1",
        "kiwi",
        "gitlab.example.com/g/kiwi",
        "explorer/alpha",
    ] {
        assert!(texts.iter().any(|one| one == said), "{said}: {texts:?}");
    }
    let worktree = Target::Worktree(groove_types::WorktreeId::new("wt-1"));
    assert!(hits.rect_of(&worktree).is_some(), "its row selects it");
    assert!(!texts.iter().any(|one| one.contains("comes to this layout")));
}

#[test]
fn the_overview_s_sidebar_heads_on_the_session_s_kind_and_folds() {
    let app = talking(Vec::new());
    let mut ui = overview();
    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(all_texts(&frame).iter().any(|one| one == "EXPLORER"));
    let head = hits
        .rect_of(&Target::AsideFold)
        .expect("the sidebar's head");
    crate::tests::click(head, &mut ui, &app, &hits);
    assert!(ui.session.folded);
}

#[test]
fn a_card_s_head_folds_the_card_on_the_overview() {
    let app = talking(Vec::new());
    let mut ui = overview();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let repo = app.session.selected().expect("a session").repos[0]
        .id
        .as_str()
        .to_string();
    let head = hits
        .rect_of(&Target::CardFold(repo))
        .expect("the repo's head");
    crate::tests::click(head, &mut ui, &app, &hits);
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let worktree = Target::Worktree(groove_types::WorktreeId::new("wt-1"));
    assert!(
        hits.rect_of(&worktree).is_none(),
        "its worktree rows fold away"
    );
}
