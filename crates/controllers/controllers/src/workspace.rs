//! The `workspace` controller: one function per user action on the `workspace` service.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use groove_types::{Caret, Edit, Error, ErrorKind, Result, WorktreeId};
use groove_workspace_service::{
    Buffer, Derived, Document, Opened, changes, derived, opened, painted, reopened, summary,
};

use crate::spawn::coalesced;
use crate::{AppState, Continuation, Deliver, Services, Spawner};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `workspace.load`: the selected worktree's changed files.
    Load,
    /// `workspace.open_file`: one file's two sides and the rows between them.
    OpenFile {
        path: String,
        /// Where to put the caret, when the click that asked knows.
        at: Option<Caret>,
    },
    /// `workspace.show`: which rows of the whole change are on screen.
    Show { rows: std::ops::Range<usize> },
    /// `workspace.edit`: one keystroke on the open buffer.
    Edit(Edit),
    /// `workspace.save_file`: the buffer to the file it came from.
    SaveFile,
    /// `workspace.copy`: what the carets hold, to the clipboard.
    Copy,
    /// `workspace.cut`: the same, and out of the buffer.
    Cut,
    /// `workspace.paste`: the clipboard, over what the carets hold.
    Paste,
    /// `workspace.stage`: one path into the index.
    Stage { path: String },
    /// `workspace.unstage`: one path back out of it.
    Unstage { path: String },
    /// `workspace.discard`: what one path holds, thrown away.
    Discard { path: String },
    /// `workspace.message`: one keystroke on the commit message.
    Message(Edit),
    /// `workspace.commit`: the index, with the message the box holds.
    Commit,
    /// `workspace.push`: the branch to its own name on origin.
    Push,
    /// `workspace.pull`: origin's own head, fast-forward only.
    Pull,
    /// `workspace.rebase`: the branch replayed on the ref it forks from.
    Rebase,
    /// `workspace.discard_all`: every change in the worktree, thrown away.
    DiscardAll,
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Load => "workspace.load",
            Command::OpenFile { .. } => "workspace.open_file",
            Command::Show { .. } => "workspace.show",
            Command::Edit(_) => "workspace.edit",
            Command::SaveFile => "workspace.save_file",
            Command::Copy => "workspace.copy",
            Command::Cut => "workspace.cut",
            Command::Paste => "workspace.paste",
            Command::Stage { .. } => "workspace.stage",
            Command::Unstage { .. } => "workspace.unstage",
            Command::Discard { .. } => "workspace.discard",
            Command::Message(_) => "workspace.message",
            Command::Commit => "workspace.commit",
            Command::Push => "workspace.push",
            Command::Pull => "workspace.pull",
            Command::Rebase => "workspace.rebase",
            Command::DiscardAll => "workspace.discard_all",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::Load => reread(state, spawner),
        Command::OpenFile { path, at } => open_file(state, spawner, path, at),
        Command::Show { rows } => show(state, spawner, rows),
        Command::Edit(edit) => edit_file(state, spawner, edit),
        Command::SaveFile => save_file(state, spawner),
        Command::Copy => copy(state, services, spawner, false),
        Command::Cut => copy(state, services, spawner, true),
        Command::Paste => paste(state, services, spawner),
        Command::Stage { path } => index(state, spawner, Act::Stage, path),
        Command::Unstage { path } => index(state, spawner, Act::Unstage, path),
        Command::Discard { path } => index(state, spawner, Act::Discard, path),
        Command::Message(edit) => state.workspace.message.edit(&edit),
        Command::Commit => commit(state, spawner),
        Command::Push => remote(state, spawner, Remote::Push),
        Command::Pull => remote(state, spawner, Remote::Pull),
        Command::Rebase => remote(state, spawner, Remote::Rebase),
        Command::DiscardAll => discard_all(state, spawner),
    }
}

/// The rows on screen: their files take their colours, the others give theirs up.
fn show(state: &mut AppState, spawner: &dyn Spawner, rows: std::ops::Range<usize>) {
    if state.workspace.showing == rows {
        return;
    }
    state.workspace.showing = rows.clone();
    let wanted = state.workspace.over(rows);
    state
        .workspace
        .coloured
        .retain(|path, _| wanted.contains(path));
    let missing: Vec<String> = wanted
        .into_iter()
        .filter(|path| !state.workspace.coloured.contains_key(path))
        .collect();
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    if missing.is_empty() {
        return;
    }
    spawner.spawn(Box::pin(async move {
        let read = painted(&dir, missing).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.workspace.coloured.extend(read);
        }) as Continuation
    }));
}

/// One keystroke on the buffer; its rows and colours follow in a job.
fn edit_file(state: &mut AppState, spawner: &dyn Spawner, edit: Edit) {
    let Some(open) = state.workspace.opened.as_mut() else {
        return;
    };
    let before = open.new.revision();
    open.new.edit(&edit);
    if open.new.revision() != before {
        derive(state, spawner);
    }
}

/// Reads the colours and the alignment again, one read at a time: the last
/// revision wins, and a read that lands stale starts the next one.
fn derive(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(open) = state.workspace.opened.as_ref() else {
        return;
    };
    if state.workspace.deriving.is_some() {
        return;
    }
    let revision = open.new.revision();
    let (path, old, new) = (
        open.path.clone(),
        open.old.clone(),
        open.new.document().clone(),
    );
    state.workspace.deriving = Some(revision);
    spawner.spawn(Box::pin(async move {
        let read = derived(&old, new);
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.workspace.deriving = None;
                took(state, path, read, revision);
                if state
                    .workspace
                    .opened
                    .as_ref()
                    .is_some_and(|open| open.new.revision() != revision)
                {
                    derive(state, spawner);
                }
            },
        ) as Continuation
    }));
}

/// Installs what the read found, when the buffer is still the one it read.
fn took(state: &mut AppState, path: String, read: Derived, revision: u64) {
    let Some(open) = state.workspace.opened.as_mut() else {
        return;
    };
    if open.path != path || !open.new.settled(read.settled, revision) {
        return;
    }
    open.rows = read.rows;
    open.marks = read.marks;
}

/// Writes the buffer out. The watcher's read of our own write finds it clean.
fn save_file(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(open) = state.workspace.opened.as_ref() else {
        return;
    };
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let (path, text) = (open.path.clone(), open.new.text());
    let job = state.begin(format!("saving {path}"));
    spawner.spawn(Box::pin(async move {
        let written = groove_workspace_service::save(&dir, &path, &text);
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match written {
                Ok(()) => saved(state, &path),
                Err(e) => state.errors.push(e),
            }
        }) as Continuation
    }));
}

/// The buffer owes the disk nothing, so a later read may replace it.
fn saved(state: &mut AppState, path: &str) {
    if let Some(open) = state
        .workspace
        .opened
        .as_mut()
        .filter(|open| open.path == path)
    {
        open.new.saved();
    }
}

/// What the carets hold, to the clipboard, off the main thread.
fn copy(state: &mut AppState, services: &Services, spawner: &dyn Spawner, cut: bool) {
    let Some(open) = state.workspace.opened.as_ref() else {
        return;
    };
    let held = open.new.selected();
    if held.is_empty() {
        return;
    }
    if cut {
        edit_file(state, spawner, Edit::Delete);
    }
    let clipboard = services.clipboard.clone();
    spawner.spawn(Box::pin(async move {
        let written = clipboard.write(&held);
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if let Err(e) = written {
                state.errors.push(Error::new(ErrorKind::Io, e.to_string()));
            }
        }) as Continuation
    }));
}

/// The clipboard read in a job, then put in where the carets are.
fn paste(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    if state.workspace.opened.is_none() {
        return;
    }
    let clipboard = services.clipboard.clone();
    spawner.spawn(Box::pin(async move {
        let text = clipboard.read();
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                if let Some(text) = text.filter(|text| !text.is_empty()) {
                    edit_file(state, spawner, Edit::Insert(text));
                }
            },
        ) as Continuation
    }));
}

/// What one action does against the remote or the base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Remote {
    Push,
    Pull,
    Rebase,
}

impl Remote {
    fn label(self) -> &'static str {
        match self {
            Remote::Push => "pushing",
            Remote::Pull => "pulling",
            Remote::Rebase => "rebasing",
        }
    }
}

/// One action against the remote. HEAD may move, so everything is read again.
fn remote(state: &mut AppState, spawner: &dyn Spawner, act: Remote) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let Some(worktree) = state
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .cloned()
    else {
        return;
    };
    let job = state.begin(act.label());
    spawner.spawn(Box::pin(async move {
        let done = match act {
            Remote::Push => groove_workspace_service::push(&dir, &worktree.branch).await,
            Remote::Pull => groove_workspace_service::pull(&dir).await,
            Remote::Rebase => {
                groove_workspace_service::rebase(&dir, worktree.base_ref.clone()).await
            }
        };
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                if let Err(e) = done {
                    state.errors.push(e);
                }
                crate::session::refresh_status(state, services, spawner);
                reread(state, spawner);
            },
        ) as Continuation
    }));
}

/// Every change in the worktree, thrown away.
fn discard_all(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let paths: Vec<String> = state
        .workspace
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect();
    if paths.is_empty() {
        return;
    }
    let job = state.begin("discarding every change");
    spawner.spawn(Box::pin(async move {
        let done = groove_workspace_service::discard(&dir, &paths).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match done {
                    Ok(()) => load(state, spawner),
                    Err(e) => state.errors.push(e),
                }
            },
        ) as Continuation
    }));
}

/// What one action does to the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Act {
    Stage,
    Unstage,
    Discard,
}

impl Act {
    fn label(self) -> &'static str {
        match self {
            Act::Stage => "staging",
            Act::Unstage => "unstaging",
            Act::Discard => "discarding",
        }
    }
}

/// One path into the index, out of it, or thrown away.
fn index(state: &mut AppState, spawner: &dyn Spawner, act: Act, path: String) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let job = state.begin(format!("{} {path}", act.label()));
    spawner.spawn(Box::pin(async move {
        let paths = [path];
        let done = match act {
            Act::Stage => groove_workspace_service::stage(&dir, &paths).await,
            Act::Unstage => groove_workspace_service::unstage(&dir, &paths).await,
            Act::Discard => groove_workspace_service::discard(&dir, &paths).await,
        };
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match done {
                    Ok(()) => load(state, spawner),
                    Err(e) => state.errors.push(e),
                }
            },
        ) as Continuation
    }));
}

/// The index committed, then every side of the diff read again.
fn commit(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let message = state.workspace.message.text();
    if message.trim().is_empty() {
        return;
    }
    let job = state.begin("committing");
    spawner.spawn(Box::pin(async move {
        let done = groove_workspace_service::commit(&dir, message.trim()).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match done {
                    Ok(()) => committed(state, spawner),
                    Err(e) => state.errors.push(e),
                }
            },
        ) as Continuation
    }));
}

/// The message is spent, and every side of the diff is read again.
fn committed(state: &mut AppState, spawner: &dyn Spawner) {
    state.workspace.message = Buffer::default();
    reread(state, spawner);
}

/// Where the HEAD side of a reopened file comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Head {
    /// Read it again: a git command may have moved it.
    Read,
    /// Keep the one in hand: only the working side changed.
    Keep,
}

/// The worktree and the file it is showing, read at the same time: the file's rows do
/// not wait on the summary.
fn reread(state: &mut AppState, spawner: &dyn Spawner) {
    load(state, spawner);
    reopen(state, spawner, Head::Read);
}

/// The same, for what a write under the worktree touched.
fn refresh(state: &mut AppState, services: &Services, spawner: &dyn Spawner, paths: &[PathBuf]) {
    if state.workspace.moved_git(paths) {
        crate::session::refresh_status(state, services, spawner);
        return reread(state, spawner);
    }
    load(state, spawner);
    if state.workspace.shows(paths) {
        reopen(state, spawner, Head::Keep);
    }
}

/// Reads the open file again, now the worktree has moved under it. A buffer with
/// unsaved edits is left alone: the user's text outranks the disk's.
fn reopen(state: &mut AppState, spawner: &dyn Spawner, head: Head) {
    let Some(open) = state.workspace.opened.as_ref() else {
        return;
    };
    if open.new.dirty() {
        return;
    }
    let path = open.path.clone();
    let old = match head {
        Head::Keep => Some(open.old.clone()),
        Head::Read => None,
    };
    read(state, spawner, path, old, None);
}

/// Reads both sides in a job; the continuation stores them for the tab to draw.
pub fn open_file(state: &mut AppState, spawner: &dyn Spawner, path: String, at: Option<Caret>) {
    read(state, spawner, path, None, at);
}

fn read(
    state: &mut AppState,
    spawner: &dyn Spawner,
    path: String,
    old: Option<Document>,
    at: Option<Caret>,
) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let job = state.begin(format!("opening {path}"));
    spawner.spawn(Box::pin(async move {
        let file = sides(&dir, &path, old).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match file {
                Ok(file) => arrived(state, file, at),
                Err(e) => state.errors.push(e),
            }
        }) as Continuation
    }));
}

/// The file read, with the caret it had while it was the same file.
fn arrived(state: &mut AppState, mut file: Opened, at: Option<Caret>) {
    let caret = at.or_else(|| {
        state
            .workspace
            .opened
            .as_ref()
            .filter(|open| open.path == file.path)
            .map(|open| open.new.caret())
    });
    if let Some(caret) = caret {
        file.new.follow(caret);
    }
    state.workspace.opened = Some(file);
}

/// The file's two sides, reading HEAD only when the old one is not in hand.
async fn sides(dir: &Path, path: &str, old: Option<Document>) -> Result<Opened> {
    match old {
        Some(old) => Ok(reopened(dir, path, old)),
        None => opened(dir, path).await,
    }
}

/// Reads the summary in a job; the continuation stores it against its worktree.
pub fn load(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(worktree) = state
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
    else {
        return;
    };
    let (id, dir) = (worktree.id.clone(), PathBuf::from(&worktree.path));
    let job = state.begin("changed files");
    spawner.spawn(Box::pin(async move {
        let files = summary(&dir).await;
        let read = match &files {
            Ok(files) => changes(&dir, files).await,
            Err(_) => Default::default(),
        };
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match files {
                Ok(files) => state.workspace.loaded(id, files, read),
                Err(e) => state.errors.push(e),
            }
        }) as Continuation
    }));
}

/// The selected worktree read and watched, or nothing when none is selected.
pub fn follow(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(worktree) = selected(state) else {
        return state.workspace.clear();
    };
    if stale(state) {
        state.workspace.opened = None;
        load(state, spawner);
    }
    if state.workspace.watching.as_ref() != Some(&worktree) {
        watch(state, spawner, worktree);
    }
}

fn watch(state: &mut AppState, spawner: &dyn Spawner, worktree: WorktreeId) {
    let Some(dir) = directory(state, &worktree) else {
        return;
    };
    spawner.spawn(Box::pin(async move {
        let git = groove_workspace_service::git_dir(&dir).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                let on_change = reload(spawner.sink(), worktree.clone());
                let watched = groove_workspace_service::watch(
                    &mut state.workspace,
                    worktree,
                    &dir,
                    git,
                    on_change,
                );
                if let Err(e) = watched {
                    state.errors.push(e);
                }
            },
        ) as Continuation
    }));
}

/// One read in flight at most. What moves while it runs waits for the next one.
fn reload(
    sink: Arc<dyn Deliver>,
    worktree: WorktreeId,
) -> impl Fn(Vec<PathBuf>) + Send + Sync + 'static {
    let changed: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
    let queue = changed.clone();
    let once = coalesced(sink, move || {
        let (worktree, changed) = (worktree.clone(), changed.clone());
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                let paths = std::mem::take(&mut *lock(&changed));
                if state.workspace.watching.as_ref() == Some(&worktree) {
                    refresh(state, services, spawner, &paths);
                }
            },
        ) as Continuation
    });
    move |paths: Vec<PathBuf>| {
        lock(&queue).extend(paths);
        once();
    }
}

fn lock(paths: &Mutex<Vec<PathBuf>>) -> std::sync::MutexGuard<'_, Vec<PathBuf>> {
    paths.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Where the selected worktree sits on disk.
fn worktree_dir(state: &AppState) -> Option<PathBuf> {
    let open = state.session.selected()?;
    Some(PathBuf::from(&open.selected_worktree()?.path))
}

fn directory(state: &AppState, worktree: &WorktreeId) -> Option<PathBuf> {
    let open = state.session.selected()?;
    let found = open.worktrees.iter().find(|w| &w.id == worktree)?;
    Some(PathBuf::from(&found.path))
}

pub fn stale(state: &AppState) -> bool {
    match selected(state) {
        Some(worktree) => !state.workspace.holds(&worktree),
        None => false,
    }
}

fn selected(state: &AppState) -> Option<WorktreeId> {
    let open = state.session.selected()?;
    Some(open.selected_worktree()?.id.clone())
}

pub fn loaded_for(state: &AppState) -> Option<&WorktreeId> {
    state.workspace.worktree.as_ref()
}
