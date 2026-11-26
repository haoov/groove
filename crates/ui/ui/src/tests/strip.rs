//! The workspace's strip: a click on the open tab folds it, any tab opens it again.

use groove_gfx::Fonts;

use crate::hit::Target;
use crate::layout::Layout;
use crate::tests::{click, full_app, window};
use crate::views::session::Tab;
use crate::{Ui, view};

fn tab_click(tab: Tab, ui: &mut Ui, app: &groove_controllers::AppState) {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let rect = hits.rect_of(&Target::Tab(tab)).expect("the tab");
    click(rect, ui, app, &hits);
}

#[test]
fn a_click_on_the_open_tab_folds_the_strip_and_widens_the_agent() {
    let app = full_app();
    let mut ui = Ui::default();
    let open = Layout::of(window(), &ui);
    tab_click(Tab::Overview, &mut ui, &app);
    assert!(ui.session.stowed);
    assert_eq!(
        ui.focus,
        crate::Focus::Agent,
        "the keys go back to the agent"
    );
    let folded = Layout::of(window(), &ui);
    assert_eq!(folded.strip.h, folded.header.h, "the header alone");
    assert_eq!(folded.strip.right(), open.strip.right());
    assert!(folded.workspace.is_empty() && folded.sidebar.is_empty());
    assert_eq!(folded.agent.w, folded.board.w, "the agent takes the ground");
    assert_eq!(folded.agent.y, open.agent.y, "under the top bar still");
}

#[test]
fn any_tab_opens_the_folded_strip_on_itself() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.stowed = true;
    tab_click(Tab::Diff, &mut ui, &app);
    assert!(!ui.session.stowed);
    assert_eq!(ui.session.tab, Tab::Diff);
    assert!(!Layout::of(window(), &ui).workspace.is_empty());
}

#[test]
fn the_strip_stands_as_an_opaque_panel_on_the_agent_s_ground() {
    let app = full_app();
    let ui = Ui::default();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let layout = Layout::of(window(), &ui);
    let base = &frame.layers()[0];
    assert!(base.under.iter().any(|(rect, ..)| *rect == layout.board));
    let panes = crate::tests::panes(&frame, &app);
    assert!(
        panes.iter().any(|one| one.1 == layout.strip),
        "its own opaque pane"
    );
    assert!(
        layout.header.y > layout.strip.y - 1.0 && layout.header.right() <= layout.strip.right()
    );
}

#[test]
fn folding_the_drawer_leaves_the_strip_folded() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.stowed = true;
    for _ in 0..2 {
        let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
        let fold = hits.rect_of(&Target::ShellFold).expect("the drawer's fold");
        click(fold, &mut ui, &app, &hits);
        ui.settle(&app);
        assert!(ui.session.stowed, "manual {}", ui.session.manual);
    }
    let terminals = || crate::palette::Act::Key(crate::keymap::Action::Terminals);
    crate::input::acted(terminals(), &mut ui, &app);
    assert!(ui.session.manual && ui.focus == crate::Focus::Terminal);
    crate::input::acted(terminals(), &mut ui, &app);
    ui.settle(&app);
    assert!(
        !ui.session.manual && ui.session.stowed,
        "the chord folds it back too"
    );
    assert_eq!(ui.focus, crate::Focus::Agent);
}

#[test]
fn the_folded_strip_is_a_capsule_as_narrow_as_its_title_and_tabs() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.stowed = true;
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let layout = Layout::of(window(), &ui);
    let capsule = crate::tests::panes(&frame, &app)
        .into_iter()
        .map(|one| one.1)
        .find(|one| one.y == layout.header.y && one.x > layout.rail.right())
        .expect("the strip's pane");
    assert_eq!(capsule.right(), layout.strip.right(), "at the right end");
    assert!(capsule.w < layout.strip.w, "only what it holds");
    let title = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text == "Alpha" && run.y < layout.header.bottom())
        .expect("the title, whole");
    assert!(title.x > capsule.x);
    let tab = hits.rect_of(&Target::Tab(Tab::Overview)).expect("a tab");
    assert!(tab.x > title.x, "the tabs after the title");
}

#[test]
fn an_icon_alone_is_a_circle_under_the_pointer() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    for target in [Target::Refresh, Target::Fold, Target::ShellFold] {
        ui.hover = Some(target.clone());
        let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
        let at = hits.rect_of(&target).expect("the button");
        let ground = frame.layers()[0]
            .quads
            .iter()
            .find(|quad| at.contains(quad.rect.x + 1.0, quad.rect.y + 1.0) && quad.rect.w <= at.w)
            .unwrap_or_else(|| {
                panic!(
                    "{target:?} lit at {at:?}: {:?}",
                    frame.layers()[0]
                        .quads
                        .iter()
                        .filter(|q| q.rect.intersect(at).w > 0.0)
                        .collect::<Vec<_>>()
                )
            });
        assert_eq!(
            ground.rect.w, ground.rect.h,
            "{target:?} square: {:?}",
            ground.rect
        );
        assert_eq!(ground.shape.radius, ground.rect.h / 2.0, "{target:?} round");
    }
}

#[test]
fn a_flat_button_lights_on_a_rounded_ground_and_a_worktree_row_on_a_square_one() {
    let app = full_app();
    let mut ui = Ui::default();
    let tokens = window().tokens();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
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
        let lit = frame.layers()[0]
            .quads
            .iter()
            .any(|quad| quad.rect == at && quad.shape.radius == round);
        assert!(lit, "{target:?} lit, its corners {round}");
    }
    let new = hits.rect_of(&Target::ShellNew).expect("the + button");
    assert_eq!(new.w, new.h, "an icon button, square around its circle");
}

#[test]
fn a_tab_keeps_its_width_whether_open_or_not() {
    let app = full_app();
    let mut ui = Ui::default();
    let width = |ui: &Ui| {
        let (_, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        hits.rect_of(&Target::Tab(Tab::Diff)).expect("the tab").w
    };
    let closed = width(&ui);
    ui.session.tab = Tab::Diff;
    assert_eq!(
        width(&ui),
        closed,
        "the open tab's text is the same size and weight"
    );
}

#[test]
fn the_first_file_s_head_holds_at_the_top_from_the_first_pixel_scrolled() {
    let app = crate::tests::diff::opened();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        hits.rect_of(&Target::Pinned).is_none(),
        "unscrolled, the head stands in its row"
    );
    ui.session.diff = 1.0;
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let pinned = hits.rect_of(&Target::Pinned).expect("held at once");
    let glassed = frame.layers().iter().any(|layer| {
        layer
            .glass
            .iter()
            .any(|one| pinned.contains(one.rect.x, one.rect.y + 1.0))
    });
    assert!(
        glassed,
        "the held head stands on glass, nothing showing through"
    );
}

#[test]
fn a_file_s_band_reads_from_the_left_its_counts_right_after_its_path() {
    let app = crate::tests::diff::opened();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Files;
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let workspace = Layout::of(window(), &ui).workspace;
    let texts = &frame.layers()[0].texts;
    let path = texts
        .iter()
        .find(|run| run.text == "src/lib.rs" && workspace.contains(run.x, run.y + 1.0))
        .expect("the path");
    let count = texts
        .iter()
        .find(|run| run.text.starts_with('+') && (run.y - path.y).abs() < 1.0)
        .expect("its counts on its line");
    assert!(count.x > path.x, "after the path");
    assert!(
        count.x < workspace.x + workspace.w / 2.0,
        "not pushed to the right"
    );
}

#[test]
fn a_note_being_written_stands_on_glass_as_a_read_one_does() {
    let app = crate::tests::diff::opened();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Files;
    let anchor = groove_types::Anchor::line("src/lib.rs", 0);
    ui.session.noting = Some(crate::views::session::Noting::new(anchor));
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let panes = frame.layers()[1..]
        .iter()
        .flat_map(|layer| layer.glass.iter())
        .filter(|one| code.contains(one.rect.x, one.rect.y))
        .count();
    assert_eq!(panes, 1, "the composer floats over its rows, one pane");
    let sliced = frame.layers()[0].quads.iter().any(|quad| {
        code.contains(quad.rect.x, quad.rect.y)
            && quad.shape.radius > 0.0
            && quad.rect.w > code.w / 2.0
    });
    assert!(!sliced, "no card cut row by row among the code");
}

/// The layer the agent's usage line draws on.
fn conversation_layer(frame: &groove_gfx::Frame) -> usize {
    let mut layers = frame.layers().iter();
    let said = |text: &str| text.starts_with("The conversation shows here");
    layers
        .position(|one| one.texts.iter().any(|run| said(&run.text)))
        .expect("the agent's conversation")
}

#[test]
fn the_folded_header_stands_a_layer_over_the_agent_s_conversation() {
    let mut app = crate::tests::agent::asking(Vec::new());
    crate::tests::agent::headless(&mut app);
    let ui = Ui {
        surface: crate::Surface::Session,
        session: crate::views::session::SessionUi {
            stowed: true,
            ..Default::default()
        },
        ..Ui::default()
    };
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let layout = Layout::of(window(), &ui);
    assert!(
        layout.header.bottom() <= layout.agent.y,
        "the capsule stands in the room over the agent's conversation"
    );
    let capsule = crate::tests::panes(&frame, &app)
        .into_iter()
        .find(|(_, one)| one.y == layout.header.y && one.right() == layout.header.right());
    assert!(
        capsule.map(|one| one.0) > Some(conversation_layer(&frame)),
        "its pane is a layer over the conversation, not under its text"
    );
    let tab = hits.rect_of(&Target::Tab(Tab::Diff)).expect("a tab");
    assert_eq!(
        hits.at(tab.x + 1.0, tab.y + 1.0),
        Some(Target::Tab(Tab::Diff))
    );
}
