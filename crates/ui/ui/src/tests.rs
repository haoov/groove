mod agent;
mod appearance;
mod asks;
mod attaching;
mod bar;
mod board;
mod budget;
mod clusters;
mod commit;
mod diff;
mod editing;
mod feed;
mod field;
mod files;
mod finding;
mod first_run;
mod focus;
mod frame;
mod keymap;
mod manual;
mod mapping;
mod mouse;
mod overview;
mod painted;
mod palette;
mod palette_look;
mod palette_more;
mod pasting;
mod pending;
mod perf;
mod rail;
mod regions;
mod resources;
mod routine_session;
mod routines;
mod scoping;
mod settings;
mod staging;
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

/// Groove's own ring, and its shifted half.
const ALT: Modifiers = Modifiers {
    ctrl: false,
    shift: false,
    alt: true,
};
const ALT_SHIFT: Modifiers = Modifiers {
    ctrl: false,
    shift: true,
    alt: true,
};
const CTRL_SHIFT: Modifiers = Modifiers {
    ctrl: true,
    shift: true,
    alt: false,
};

/// A key, with nothing drawn: a key never reads the regions.
fn press(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let input = Input::Key { key, mods };
    handle(input, ui, app, &Hits::default(), window())
}

/// Each character of `text` as a key; what the last one asked.
fn typed(text: &str, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let mut last = Vec::new();
    for c in text.chars() {
        last = press(Key::Char(c), Modifiers::default(), ui, app);
    }
    last
}

/// Each character of `text` as a key in the palette.
fn typed_into(palette: &mut crate::palette::Palette, text: &str, app: &AppState) {
    for c in text.chars() {
        palette.key(Key::Char(c), app);
    }
}

/// A click in the middle of what was drawn there.
fn click(rect: Rect, ui: &mut Ui, app: &AppState, hits: &Hits) -> Vec<Command> {
    let input = Input::Press {
        x: rect.x + rect.w / 2.0,
        y: rect.y + rect.h / 2.0,
        mods: Default::default(),
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
    handle(
        Input::Press {
            x,
            y,
            mods: Default::default(),
        },
        ui,
        app,
        hits,
        window(),
    )
}

fn release(ui: &mut Ui, app: &AppState, hits: &Hits) -> Vec<Command> {
    handle(Input::Release, ui, app, hits, window())
}

/// What the bottom layer of a frame says, in draw order.
fn texts_of(frame: &groove_gfx::Frame) -> Vec<String> {
    frame.layers()[0]
        .texts
        .iter()
        .map(|run| run.text.clone())
        .collect()
}

/// What every layer of a frame says, bottom layer first.
fn all_texts(frame: &groove_gfx::Frame) -> Vec<String> {
    frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter().map(|run| run.text.clone()))
        .collect()
}

/// One frame drawn: what its bottom layer says, and its hits.
fn render(app: &AppState, ui: &Ui, metrics: Metrics) -> (Vec<String>, Hits) {
    let (frame, hits) = crate::view(app, ui, metrics, &mut groove_gfx::Fonts::embedded());
    (texts_of(&frame), hits)
}

/// The frame's texts, a line drawn in lit pieces read back whole.
fn whole_texts(runs: &[groove_gfx::TextRun]) -> Vec<String> {
    let mut out: Vec<(String, f32, groove_gfx::Rect)> = Vec::new();
    for run in runs {
        match out.last_mut() {
            Some((text, y, clip)) if *y == run.y && *clip == run.clip => text.push_str(&run.text),
            _ => out.push((run.text.clone(), run.y, run.clip)),
        }
    }
    out.into_iter().map(|(text, ..)| text).collect()
}

/// The open file, and the whole change it belongs to.
fn shows(app: &mut AppState, path: &str, before: &str, after: &str) {
    use groove_controllers::workspace_service::from_text;
    open_file(app, from_text(path, before, after));
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

pub(crate) fn metrics(w: u32, h: u32, scale: f32) -> Metrics {
    let design = crate::Tokens::new(1.0);
    Metrics {
        size: Size::new(w, h),
        scale,
        text: design.text,
        code: design.code,
        terminal: design.code,
        cell: CellSize {
            width: 8.0 * scale,
            height: 17.0 * scale,
        },
        advance: groove_gfx::Fonts::embedded().advance(design.code * scale),
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
        status: Default::default(),
        read: Default::default(),
        clusters: vec![],
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
        .add_worktree(repo, wt.clone());
    app.session.living = app
        .session
        .open
        .iter()
        .map(|open| groove_controllers::session_service::Living {
            session: open.session.clone(),
            worktrees: open.worktrees.clone(),
            repos: open.repos.len(),
        })
        .collect();
    app
}

fn full_app() -> AppState {
    with_repo(app())
}

fn task(short_id: &str, title: &str, external: &str) -> groove_types::Task {
    groove_types::Task {
        external_id: groove_types::ExternalId::new(external),
        short_id: short_id.to_string(),
        title: title.to_string(),
        status: "In progress".into(),
        intent: Some(groove_types::StatusIntent::InProgress),
        priority: Some(groove_types::Priority::High),
        dates: groove_types::TaskDates::default(),
        estimate: Some(4.0),
        logged: Some(1.5),
        synced_at: Timestamp::now(),
        provider: groove_types::ProviderId::Github,
        url: None,
        project: Some("Platform".into()),
        branch_tag: Some("50".into()),
    }
}

/// A file open for editing in the worktree the workspace holds, and active.
fn open_file(app: &mut AppState, file: groove_controllers::workspace_service::Opened) {
    let worktree = match app.workspace.worktree.clone() {
        Some(one) => one,
        None => {
            let one = WorktreeId::new("w");
            app.workspace
                .loaded(one.clone(), Vec::new(), Default::default());
            one
        }
    };
    app.workspace.arrived(&worktree, file, None, true);
}

/// The session surface on the tab and view that draw this face.
fn set_face(ui: &mut Ui, face: crate::views::session::Face) {
    use crate::views::session::{Face, Tab};
    match face {
        Face::File => ui.session.tab = Tab::Files,
        Face::Stream(view) => {
            ui.session.tab = Tab::Diff;
            ui.session.view = view;
        }
    }
}
