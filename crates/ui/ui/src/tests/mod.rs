mod budget;
mod diff;
mod editing;
mod field;
mod files;
mod finding;
mod focus;
mod frame;
mod index;
mod mouse;
mod painted;
mod palette;
mod perf;
mod rail;
mod structure;

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command};
use groove_gfx::{CellSize, Rect, Size};
use groove_types::{
    FileDiff, FileStatus, PoolEntry, Repo, RepoId, Session, SessionId, SessionKind, SessionState,
    Timestamp, Worktree, WorktreeId,
};

use crate::hit::Hits;
use crate::input::{Input, Key, Modifiers, handle};
use crate::{Metrics, Ui};

/// The window every mouse test works in.
const WINDOW: (u32, u32) = (1280, 800);

/// Groove's own modifier pair.
const CHORD: Modifiers = Modifiers {
    ctrl: true,
    shift: true,
    alt: false,
};

/// A key, with nothing drawn: a key never reads the regions.
fn press(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let input = Input::Key { key, mods };
    handle(input, ui, app, &Hits::default(), window())
}

/// A click in the middle of what was drawn there.
fn click(rect: Rect, ui: &mut Ui, app: &AppState, hits: &Hits) -> Vec<Command> {
    let input = Input::Press {
        x: rect.x + rect.w / 2.0,
        y: rect.y + rect.h / 2.0,
    };
    handle(input, ui, app, hits, window())
}

/// The pointer at a physical x, mid-drag.
fn drag(x: f32, ui: &mut Ui, app: &AppState, hits: &Hits) -> Vec<Command> {
    handle(Input::Move { x, y: 0.0 }, ui, app, hits, window())
}

/// The pointer at a point, mid-drag.
fn drag_at(x: f32, y: f32, ui: &mut Ui, app: &AppState, hits: &Hits) -> Vec<Command> {
    handle(Input::Move { x, y }, ui, app, hits, window())
}

/// A press at a point.
fn pressed(x: f32, y: f32, ui: &mut Ui, app: &AppState, hits: &Hits) -> Vec<Command> {
    handle(Input::Press { x, y }, ui, app, hits, window())
}

fn release(ui: &mut Ui, app: &AppState, hits: &Hits) -> Vec<Command> {
    handle(Input::Release, ui, app, hits, window())
}

/// The open file, and the whole change it belongs to.
fn shows(app: &mut AppState, path: &str, before: &str, after: &str) {
    use groove_controllers::workspace_service::from_text;
    app.workspace.opened = Some(from_text(path, before, after));
    changed_files(app, &[(path, before, after)]);
}

/// Several changed files, as one change with nothing open.
fn changed_files(app: &mut AppState, files: &[(&str, &str, &str)]) {
    use groove_controllers::workspace_service::{Changes, aligned};
    let read = files
        .iter()
        .map(|(path, before, after)| aligned(path, before, after))
        .collect();
    app.workspace.changes = Changes::new(read);
    app.workspace.files = files
        .iter()
        .map(|(path, _, _)| FileDiff {
            path: (*path).to_string(),
            added: 1,
            deleted: 1,
            status: FileStatus::Modified,
            staged: Some(false),
        })
        .collect();
}

/// The sidebar with the keyboard, for the bar's own clicks.
fn sidebar_ui() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = crate::views::session::Tab::Diff;
    ui.focus = crate::Focus::Sidebar;
    ui
}

fn window() -> Metrics {
    metrics(WINDOW.0, WINDOW.1, 1.0)
}

fn metrics(w: u32, h: u32, scale: f32) -> Metrics {
    let design = crate::Tokens::new(1.0);
    Metrics {
        size: Size::new(w, h),
        scale,
        text: design.text,
        code: design.code,
        cell: CellSize {
            width: 8.0 * scale,
            height: 17.0 * scale,
        },
        tick: 0,
        now: Timestamp::new(0),
    }
}

fn open(id: &str, title: &str) -> Open {
    Open {
        session: Session {
            id: SessionId::new(id),
            title: title.into(),
            kind: SessionKind::Explorer,
            created_at: Timestamp::new(0),
        },
        state: SessionState::default(),
        repos: vec![],
        worktrees: vec![],
        delivery: vec![],
        read: Default::default(),
    }
}

fn app() -> AppState {
    let mut app = AppState::default();
    app.session.open.push(open("a", "Alpha"));
    app.session.open.push(open("b", "Beta"));
    app.session.selected = Some(SessionId::new("a"));
    app.session.pool = vec![PoolEntry {
        slug: "gitlab.example.com/g/mayo".into(),
        path: "/pool/mayo".into(),
    }];
    app
}

fn with_repo(mut app: AppState) -> AppState {
    let repo = Repo {
        id: RepoId::new("gitlab.example.com/g/mayo"),
        host: "gitlab.example.com".into(),
        group_path: "g".into(),
        project: "mayo".into(),
        local_path: "/pool/mayo".into(),
    };
    let wt = Worktree {
        id: WorktreeId::new("wt-1"),
        session: SessionId::new("a"),
        repo: repo.id.clone(),
        branch: "explorer/alpha".into(),
        path: "/code/worktrees/a/mayo/explorer/alpha".into(),
        base_ref: None,
        created_at: Timestamp::new(0),
    };
    app.session
        .branches
        .push((repo.id.clone(), vec!["main".into(), "release/1.0".into()]));
    app.session
        .get_mut(&SessionId::new("a"))
        .unwrap()
        .add_worktree(repo, wt);
    app
}

fn full_app() -> AppState {
    with_repo(app())
}
