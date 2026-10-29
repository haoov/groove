//! The chords that move: a region and its fold, a pane's size, a list's rows, a terminal's tabs.

use groove_controllers::{AppState, Command, session, shell, workspace};

use super::*;
use crate::Focus;
use crate::input::Modifiers;

const CTRL_ALT: Modifiers = Modifiers {
    ctrl: true,
    shift: false,
    alt: true,
};

#[test]
fn alt_o_opens_the_workspace_then_gives_it_the_keys_then_folds_it() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.stowed = true;
    press(Key::Char('o'), ALT, &mut ui, &app);
    assert_eq!((ui.session.stowed, ui.focus), (false, Focus::Workspace));
    ui.focus = Focus::Agent;
    press(Key::Char('o'), ALT, &mut ui, &app);
    assert_eq!((ui.session.stowed, ui.focus), (false, Focus::Workspace));
    press(Key::Char('o'), ALT, &mut ui, &app);
    assert_eq!((ui.session.stowed, ui.focus), (true, Focus::Agent));
}

#[test]
fn alt_f_opens_the_palette_on_the_worktree_s_files() {
    let app = full_app();
    let mut ui = Ui::default();
    let asked = press(Key::Char('f'), ALT, &mut ui, &app);
    assert_eq!(asked, [Command::Workspace(workspace::Command::ListPaths)]);
    let palette = ui.palette().expect("the palette");
    assert_eq!(palette.query.text(), "#");
}

#[test]
fn alt_shift_q_closes_the_session() {
    let app = full_app();
    let mut ui = Ui::default();
    let closed = press(Key::Char('q'), ALT_SHIFT, &mut ui, &app);
    let session = app.session.selected.clone().expect("a session");
    assert_eq!(
        closed,
        [Command::Session(session::Command::Close { session })]
    );
}

#[test]
fn ctrl_alt_and_a_direction_move_the_focused_pane_s_boundary_that_way() {
    let app = full_app();
    let mut ui = Ui {
        focus: Focus::Agent,
        ..Ui::default()
    };
    let agent = ui.split.agent;
    let kept = press(Key::Char('l'), CTRL_ALT, &mut ui, &app);
    assert!(ui.split.agent > agent, "the agent's right edge goes right");
    let keep = groove_controllers::config::Command::KeepPanes(ui.split.panes());
    assert_eq!(kept, [Command::Config(keep)], "kept for the next run");
    press(Key::Char('h'), CTRL_ALT, &mut ui, &app);
    assert_eq!(ui.split.agent, agent, "and back left");
    ui.focus = Focus::Workspace;
    press(Key::Char('h'), CTRL_ALT, &mut ui, &app);
    assert!(
        ui.split.agent < agent,
        "the workspace's left edge goes left"
    );
    (ui.focus, ui.session.tab) = (Focus::Sidebar, crate::views::session::Tab::Diff);
    let sidebar = ui.split.sidebar;
    press(Key::Char('h'), CTRL_ALT, &mut ui, &app);
    assert!(
        ui.split.sidebar > sidebar,
        "the sidebar's left edge goes left"
    );
    ui.focus = Focus::Terminal;
    ui.session.manual = true;
    let manual = ui.split.manual;
    press(Key::Char('k'), CTRL_ALT, &mut ui, &app);
    assert!(ui.split.manual > manual, "the terminals rise");
}

#[test]
fn ctrl_j_and_ctrl_k_walk_a_list_as_the_arrows_do() {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char(';'), ALT_SHIFT, &mut ui, &app);
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    press(Key::Char('j'), ctrl, &mut ui, &app);
    press(Key::Char('j'), ctrl, &mut ui, &app);
    press(Key::Char('k'), ctrl, &mut ui, &app);
    assert_eq!(ui.palette().expect("the palette").selected, 1);
    assert!(
        ui.palette().expect("open").query.is_empty(),
        "nothing typed"
    );
    let rail = |key, mods| {
        let mut ui = Ui {
            focus: Focus::Rail,
            ..Ui::default()
        };
        press(key, mods, &mut ui, &app)
    };
    assert_eq!(
        rail(Key::Char('j'), ctrl),
        rail(Key::Down, Modifiers::default())
    );
}

/// Two terminal tabs, the first selected, the terminals holding the keys.
fn two_tabs() -> (AppState, Vec<u64>) {
    let mut app = full_app();
    let session = app.session.selected.clone().expect("a session");
    app.shell.reserve(&session, false);
    app.shell.reserve(&session, false);
    let shells = app.shell.shells(&session).expect("the terminals");
    let tabs: Vec<u64> = shells.tabs.iter().map(|tab| tab.id).collect();
    app.shell.select_tab(&session, tabs[0]);
    (app, tabs)
}

#[test]
fn in_a_terminal_ctrl_tab_steps_its_tabs_and_ctrl_w_closes_one() {
    let (app, tabs) = two_tabs();
    let session = app.session.selected.clone().expect("a session");
    let mut ui = Ui {
        focus: Focus::Terminal,
        ..Ui::default()
    };
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    let stepped = press(Key::Tab, ctrl, &mut ui, &app);
    let next = shell::Command::SelectTab {
        session: session.clone(),
        tab: tabs[1],
    };
    assert_eq!(stepped, [Command::Shell(next)]);
    let closed = press(Key::Char('w'), ctrl, &mut ui, &app);
    let close = shell::Command::CloseTab {
        session,
        tab: tabs[0],
    };
    assert_eq!(closed, [Command::Shell(close)]);
}
