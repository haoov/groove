//! What lights under the pointer, what keeps its size, and what stands on glass.

use groove_gfx::Fonts;

use crate::hit::Target;
use crate::layout::Layout;
use crate::tests::{full_app, window};
use crate::views::session::Mode;
use crate::{Ui, view};

/// The fixture's session with a terminal, its drawer's bar standing.
fn with_a_terminal() -> (groove_controllers::AppState, Ui) {
    let mut app = full_app();
    let session = app.session.selected.clone().expect("a session");
    app.shell.reserve(&session, false);
    let mut ui = Ui::default();
    ui.settle(&app);
    (app, ui)
}

#[test]
fn an_icon_alone_is_a_circle_under_the_pointer() {
    let (app, mut ui) = with_a_terminal();
    let target = Target::ShellNew;
    ui.hover = Some(target.clone());
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let at = hits.rect_of(&target).expect("the button");
    let quads = frame.layers().iter().flat_map(|layer| layer.quads.iter());
    let ground = quads
        .into_iter()
        .find(|quad| at.contains(quad.rect.x + 1.0, quad.rect.y + 1.0) && quad.rect.w <= at.w)
        .expect("lit under the pointer");
    assert_eq!(ground.rect.w, ground.rect.h, "square: {:?}", ground.rect);
    assert_eq!(ground.shape.radius, ground.rect.h / 2.0, "round");
    assert_eq!(at.w, at.h, "an icon button, square around its circle");
}

#[test]
fn a_flat_button_lights_on_a_rounded_ground_and_a_worktree_row_on_a_square_one() {
    let (app, mut ui) = with_a_terminal();
    ui.session.mode = Mode::Overview;
    let tokens = window().tokens();
    let worktree = app
        .session
        .selected()
        .and_then(|one| one.worktrees.first())
        .map(|one| one.id.clone());
    let worktree = Target::Worktree(worktree.expect("a worktree"));
    for (target, round) in [(Target::ShellSplit, tokens.glass_round), (worktree, 0.0)] {
        ui.hover = Some(target.clone());
        let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
        let at = hits.rect_of(&target).expect("drawn");
        let mut quads = frame.layers().iter().flat_map(|layer| layer.quads.iter());
        let lit = quads.any(|quad| quad.rect == at && quad.shape.radius == round);
        assert!(lit, "{target:?} lit, its corners {round}");
    }
}

#[test]
fn a_tab_keeps_its_width_whether_open_or_not() {
    let app = full_app();
    let mut ui = Ui::default();
    let width = |ui: &Ui| {
        let (_, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        let tab = hits.rect_of(&Target::ModeTab(Mode::Diff));
        tab.expect("the tab").w
    };
    let closed = width(&ui);
    ui.session.mode = Mode::Diff;
    assert_eq!(
        width(&ui),
        closed,
        "the open tab's text is the same size and weight"
    );
}

#[test]
fn a_file_s_band_reads_from_the_left_its_counts_right_after_its_path() {
    let app = crate::tests::diff::opened();
    let mut ui = Ui::default();
    ui.session.mode = Mode::Files;
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let middle = Layout::of(window(), &ui).agent;
    let texts: Vec<_> = frame
        .layers()
        .iter()
        .flat_map(|one| one.texts.iter())
        .collect();
    let path = texts
        .iter()
        .find(|run| run.text == "src/lib.rs" && middle.contains(run.x, run.y + 1.0))
        .expect("the path");
    let count = texts
        .iter()
        .find(|run| run.text.starts_with('+') && (run.y - path.y).abs() < 1.0)
        .expect("its counts on its line");
    assert!(count.x > path.x, "after the path");
    assert!(
        count.x < middle.x + middle.w / 2.0,
        "not pushed to the right"
    );
}

#[test]
fn a_reply_being_written_stands_on_glass_as_a_read_note_does() {
    let mut app = crate::tests::diff::opened();
    let mut ui = Ui::default();
    ui.session.mode = Mode::Files;
    let anchor = groove_types::Anchor::line("src/lib.rs", 0);
    app.delivery.shown = vec![crate::tests::diff::thread_on(&anchor)];
    ui.session.noting = Some(crate::views::session::Noting::reply(anchor, "t1".into()));
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let panes = frame.layers()[1..]
        .iter()
        .flat_map(|layer| layer.glass.iter())
        .filter(|one| code.contains(one.rect.x, one.rect.y))
        .count();
    assert_eq!(panes, 1, "the reply floats in its thread's card, one pane");
    let sliced = frame.layers()[0].quads.iter().any(|quad| {
        code.contains(quad.rect.x, quad.rect.y)
            && quad.shape.radius > 0.0
            && quad.rect.w > code.w / 2.0
    });
    assert!(!sliced, "no card cut row by row among the code");
}
