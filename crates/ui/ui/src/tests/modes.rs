//! The modes layout: off by default, on from Settings; the bar of title and tabs, the agent in
//! the middle, every tab built, and the terminals at the middle's foot.

use groove_gfx::Fonts;
use groove_types::chat::Heard;

use crate::hit::Target;
use crate::layout::Layout;
use crate::tests::chat::{chat_ui, talking};
use crate::tests::{all_texts, click, window};
use crate::views::session::Mode;

mod diff;
mod files;
mod kubernetes;
mod overview;

fn moded() -> crate::Ui {
    crate::Ui {
        modes: true,
        ..chat_ui()
    }
}

#[test]
fn the_switch_is_off_by_default_and_the_setting_turns_it_on() {
    let mut app = talking(Vec::new());
    let mut ui = chat_ui();
    ui.settle(&app);
    assert!(!ui.modes, "the workspace panel stays until asked");
    let config = app.config.config.as_mut().expect("a config");
    config.ui.modes = true;
    ui.settle(&app);
    assert!(ui.modes);
    assert!(!Layout::of(window(), &ui).top.is_empty());
}

#[test]
fn the_bar_holds_the_title_and_the_tabs_and_the_agent_takes_the_middle() {
    let app = talking(vec![Heard::Said("on it".into())]);
    let ui = moded();
    let layout = Layout::of(window(), &ui);
    assert!(
        layout.strip.is_empty() && layout.workspace.is_empty(),
        "no workspace panel"
    );
    assert!(
        layout.top.bottom() <= layout.agent.y,
        "the bar over the middle"
    );
    assert!(
        layout.agent.right() <= layout.sidebar.x,
        "the sidebar at the right"
    );
    assert!(
        layout.manual.y >= layout.agent.bottom(),
        "the terminals at the middle's foot"
    );
    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let texts = all_texts(&frame);
    for said in ["OVERVIEW", "AGENT", "DIFF", "FILES", "on it"] {
        assert!(texts.iter().any(|one| one == said), "{said}: {texts:?}");
    }
    assert!(hits.rect_of(&Target::ModeTab(Mode::Agent)).is_some());
}

#[test]
fn a_tab_switches_the_middle_and_no_tab_says_it_comes_next() {
    let mut app = talking(vec![Heard::Said("on it".into())]);
    let selected = app.session.selected.clone().expect("a session");
    let open = app.session.get_mut(&selected).expect("its state");
    open.clusters = vec![groove_types::Attached {
        context: "hub".into(),
        namespace: None,
    }];
    let mut ui = moded();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let kubernetes = hits
        .rect_of(&Target::ModeTab(Mode::Kubernetes))
        .expect("the Kubernetes tab");
    click(kubernetes, &mut ui, &app, &hits);
    assert_eq!(ui.session.mode, Mode::Kubernetes);
    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        !all_texts(&frame).iter().any(|one| one == "on it"),
        "the transcript is out of front"
    );
    for mode in Mode::ALL {
        ui.session.mode = mode;
        ui.settle(&app);
        let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
        let texts = all_texts(&frame);
        let pending = texts
            .iter()
            .find(|one| one.contains("comes to this layout next"));
        assert!(pending.is_none(), "{mode:?}: {pending:?}");
    }
}

#[test]
fn alt_one_to_five_pick_the_tabs_and_the_sidebar_folds_to_its_head_and_drags() {
    use crate::input::Key;
    use crate::tests::{ALT, press};
    let app = talking(vec![Heard::Said("on it".into())]);
    let mut ui = moded();
    press(Key::Char('3'), ALT, &mut ui, &app);
    assert_eq!(ui.session.mode, Mode::Diff);
    press(Key::Char('2'), ALT, &mut ui, &app);
    assert_eq!(ui.session.mode, Mode::Agent);
    let open = Layout::of(window(), &ui);
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let edge = Target::Split(crate::layout::Edge::Aside);
    assert!(hits.rect_of(&edge).is_some(), "its edge is grabbed");
    let head = hits
        .rect_of(&Target::AsideFold)
        .expect("the sidebar's head");
    click(head, &mut ui, &app, &hits);
    let folded = Layout::of(window(), &ui);
    assert_eq!(
        folded.sidebar.y, open.sidebar.y,
        "its head stays in the bar's row"
    );
    assert!(folded.sidebar.h < open.sidebar.h);
    assert!(folded.agent.w > open.agent.w, "the middle takes the width");
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(hits.rect_of(&edge).is_none(), "folded, nothing to drag");
}

#[test]
fn the_sidebar_s_edge_drags_as_the_rail_s_does() {
    use crate::tests::{drag_at, pressed, release};
    let app = talking(vec![Heard::Said("on it".into())]);
    let mut ui = moded();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let edge = Target::Split(crate::layout::Edge::Aside);
    let band = hits.rect_of(&edge).expect("the sidebar's edge");
    let before = Layout::of(window(), &ui).sidebar;
    let from = (band.x + band.w / 2.0, band.y + band.h / 2.0);
    pressed(from.0, from.1, &mut ui, &app, &hits);
    drag_at(from.0 - 60.0, from.1, &mut ui, &app, &hits);
    release(&mut ui, &app, &hits);
    let after = Layout::of(window(), &ui).sidebar;
    assert!(
        (after.x - (before.x - 60.0)).abs() < 2.0,
        "{before:?} → {after:?}"
    );
    assert_eq!(
        after.right(),
        before.right(),
        "its right edge stays at the window's"
    );
}

#[test]
fn with_both_rail_windows_folded_the_middle_takes_the_rail_s_column() {
    let mut ui = moded();
    let open = Layout::of(window(), &ui);
    (ui.rail.stowed, ui.rail.folded) = (true, true);
    let folded = Layout::of(window(), &ui);
    assert!(folded.agent.x < open.agent.x, "the middle runs left");
    assert_eq!(
        folded.agent.x, folded.sessions.x,
        "under the sessions' folded head"
    );
    assert!(
        folded.agent.bottom() <= folded.feed.y,
        "above the feed's folded head"
    );
}

#[test]
fn with_modes_the_sidebar_holds_the_model_and_the_usage_and_the_foot_stays_bare() {
    let used = Heard::Used {
        model: "claude-opus-5-5".into(),
        context: 82_002,
    };
    let mut app = talking(vec![used]);
    app.agent.agents[0].1.windows = vec![("claude-opus-5-5".into(), 1_000_000)];
    let mut ui = moded();
    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let texts = all_texts(&frame);
    assert!(
        !texts.iter().any(|one| one == "reply"),
        "no usage line at the foot"
    );
    assert!(texts.iter().any(|one| one == "opus 5.5"), "{texts:?}");
    let model = hits.rect_of(&Target::ModelButton).expect("the model's row");
    let sidebar = Layout::of(window(), &ui).sidebar;
    assert!(
        sidebar.contains(model.x + 1.0, model.y + 1.0),
        "in the sidebar"
    );
    click(model, &mut ui, &app, &hits);
    let Some(crate::Of::Models { rows, gauges, .. }) = ui.menu().map(|one| &one.of) else {
        panic!("the models' menu");
    };
    assert!(
        !rows.is_empty() && gauges.is_empty(),
        "the models alone: the sidebar has the usage"
    );
}

#[test]
fn the_terminals_bar_shows_only_with_a_terminal_and_then_runs_the_middle_s_width() {
    let mut ui = moded();
    let none = Layout::of(window(), &ui);
    assert!(none.manual.is_empty(), "no terminal, no bar");
    ui.session.terminals = true;
    let held = Layout::of(window(), &ui);
    assert!(!held.manual.is_empty());
    assert_eq!((held.manual.x, held.manual.w), (held.agent.x, held.agent.w));
    assert!(
        held.agent.bottom() < none.agent.bottom(),
        "the middle stops over it"
    );
}

#[test]
fn with_modes_the_transcript_scrolls_on_under_the_bottom_bar_and_stops_at_the_top_one() {
    let said = (0..60).map(|at| Heard::Said(format!("step {at} done")));
    let app = talking(said.collect());
    let mut ui = moded();
    ui.session.terminals = true;
    ui.agent.scroll = window().tokens().row * 10.0;
    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let layout = Layout::of(window(), &ui);
    let texts = frame.layers().iter().flat_map(|one| one.texts.iter());
    let steps: Vec<_> = texts.filter(|run| run.text.starts_with("step ")).collect();
    let pane = layout.agent;
    assert!(
        steps.iter().all(|run| run.clip.y >= pane.y),
        "none under the top bar"
    );
    assert!(
        steps.iter().any(|run| run.clip.bottom() > pane.bottom()),
        "rows run on below"
    );
}

#[test]
fn the_rail_s_folded_heads_stand_over_the_middle_as_the_sidebar_s_does() {
    let said = (0..60).map(|at| Heard::Said(format!("step {at} done")));
    let app = talking(said.collect());
    let mut ui = moded();
    (ui.rail.stowed, ui.rail.folded) = (true, true);
    ui.agent.scroll = window().tokens().row * 10.0;
    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let layout = Layout::of(window(), &ui);
    let head = layout.sessions;
    let at = hits.at(head.x + 1.0, head.y + 1.0);
    assert_eq!(
        at,
        Some(Target::SessionsFold),
        "the head answers over the middle"
    );
    let texts = frame.layers().iter().flat_map(|one| one.texts.iter());
    let mut steps = texts.filter(|run| run.text.starts_with("step "));
    let feed = layout.feed;
    assert!(
        steps.any(|run| run.clip.bottom() >= feed.y),
        "rows run on under the feed's head"
    );
}

#[test]
fn with_modes_the_agents_sent_stand_in_the_sidebar_and_no_dock_over_the_foot() {
    let sent = Heard::Subagent {
        id: "metrics".into(),
        kind: "Explore".into(),
        task: "find the metrics".into(),
    };
    let mut heard = vec![sent];
    heard.extend((0..40).map(|at| Heard::Said(format!("step {at} done"))));
    let app = talking(heard);
    let mut ui = moded();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(hits.rect_of(&Target::FoldAgents).is_none(), "no dock");
    let row = hits
        .rect_of(&Target::Subagent("metrics".into()))
        .expect("its row");
    let sidebar = Layout::of(window(), &ui).sidebar;
    assert!(sidebar.contains(row.x + 1.0, row.y + 1.0), "in the sidebar");
    click(row, &mut ui, &app, &hits);
    assert_eq!(
        ui.agent.opened,
        ["metrics"],
        "its block opened, far up the conversation"
    );
}

#[test]
fn with_modes_alt_o_gives_the_keys_to_the_middle_and_alt_e_to_the_sidebar_then_folds_it() {
    use crate::input::Key;
    use crate::tests::{ALT, press};
    let mut app = talking(Vec::new());
    app.config.config.as_mut().expect("a config").ui.modes = true;
    let mut ui = moded();
    ui.focus = crate::Focus::Rail;
    press(Key::Char('o'), ALT, &mut ui, &app);
    assert_eq!(ui.focus, crate::Focus::Agent);
    ui.session.mode = Mode::Diff;
    press(Key::Char('o'), ALT, &mut ui, &app);
    assert_eq!(ui.focus, crate::Focus::Workspace, "on DIFF, the code");
    ui.session.mode = Mode::Overview;
    press(Key::Char('e'), ALT, &mut ui, &app);
    ui.settle(&app);
    assert_eq!(
        ui.focus,
        crate::Focus::Sidebar,
        "the keys first, on a tab with nothing to pick"
    );
    assert!(!ui.session.folded);
    press(Key::Char('e'), ALT, &mut ui, &app);
    assert!(ui.session.folded, "then folded");
    assert_eq!(ui.focus, crate::Focus::Agent, "the keys back to the middle");
    press(Key::Char('e'), ALT, &mut ui, &app);
    assert!(!ui.session.folded);
    assert_eq!(ui.focus, crate::Focus::Sidebar);
}

#[test]
fn every_tab_s_sidebar_scrolls_under_one_scrollbar() {
    let sent = (0..60).map(|at| Heard::Subagent {
        id: format!("a{at}"),
        kind: "Explore".into(),
        task: format!("look {at}"),
    });
    let app = talking(sent.collect());
    let mut ui = moded();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let track = hits
        .rect_of(&Target::Thumb(crate::hit::Scroller::Aside))
        .expect("the agent's sidebar runs past its room");
    let mut low = track;
    low.y = track.bottom() - 1.0;
    low.h = 1.0;
    click(low, &mut ui, &app, &hits);
    let end = hits.extent(crate::hit::Scroller::Aside);
    assert!(end > 0.0);
    assert_eq!(ui.offset(crate::hit::Scroller::Aside), end);
}
