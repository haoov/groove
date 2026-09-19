//! What a key does, by the pane that holds the keyboard.

use groove_controllers::{AppState, Command, agent, session, workspace};

use groove_types::{Caret, DiffView, Edit, Motion, Selection};

use super::{Key, Modifiers};
use crate::field::Field;
use crate::find::Finding;
use crate::palette::Palette;
use crate::tokens::ABOVE_MATCH;
use crate::{Focus, Ui};

pub(super) fn key_input(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if mods.ctrl && mods.shift {
        return chord(key, ui, app).into_iter().collect();
    }
    if let Some(palette) = &mut ui.palette {
        let outcome = palette.key(key, app);
        if outcome.close {
            ui.palette = None;
        }
        return outcome.commands;
    }
    if ui.session.searching {
        return in_query(key, mods, ui, app);
    }
    if let Some(commands) = finding(key, mods, ui, app) {
        return commands;
    }
    if mods.ctrl && matches!(key, Key::Char('p' | 'P')) && ui.focus != Focus::Agent {
        searching(ui, true);
        return Vec::new();
    }
    match ui.focus {
        Focus::Agent => to_agent(key, mods, app).into_iter().collect(),
        Focus::Workspace => in_file(key, mods, app),
        Focus::Sidebar => in_sidebar(key, mods, ui, app),
        Focus::Rail => in_rail(key, app),
    }
}

/// The find bar's own keys, while the workspace holds the keyboard: the bar takes
/// what is typed until `Enter` hands the code back, and the chords step either way.
fn finding(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Option<Vec<Command>> {
    let view = ui.session.view;
    if mods.ctrl && matches!(key, Key::Char('f' | 'F')) && ui.focus == Focus::Workspace {
        let find = ui.session.find.get_or_insert_with(|| Finding::open(view));
        find.typing = true;
        return Some(Vec::new());
    }
    let find = ui.session.find.as_mut()?;
    match key {
        Key::Escape => {
            ui.session.find = None;
            return Some(Vec::new());
        }
        Key::Char('n' | 'N') if mods.ctrl => find.step(true),
        Key::Char('p' | 'P') if mods.ctrl => find.step(false),
        Key::Enter if find.typing => find.typing = false,
        key if find.typing => match typing(key, mods, &mut find.query) {
            true => searched(find, app, view),
            false => return Some(Vec::new()),
        },
        _ => return None,
    }
    Some(reached(ui, app))
}

/// One keystroke in a field. True when what it holds changed, which a motion does not.
fn typing(key: Key, mods: Modifiers, field: &mut Field) -> bool {
    match key {
        Key::Left => field.left(),
        Key::Right => field.right(),
        Key::Home => field.home(),
        Key::End => field.end(),
        Key::Backspace => return took(field, Field::backspace),
        Key::Delete => return took(field, Field::delete),
        Key::Char(c) if !mods.ctrl && !mods.alt => return took(field, |field| field.insert(c)),
        _ => {}
    }
    false
}

fn took(field: &mut Field, act: impl FnOnce(&mut Field)) -> bool {
    act(field);
    true
}

/// The matches read again for what the bar now holds.
fn searched(find: &mut Finding, app: &AppState, view: DiffView) {
    find.hits = crate::find::found(app, view, find.query.text());
    find.view = view;
    find.at = 0;
}

/// The surface scrolled to the match it stands on, which it holds as a selection.
fn reached(ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let line = crate::tokens::Tokens::new(1.0).line;
    let Some(find) = ui.session.find.as_ref() else {
        return Vec::new();
    };
    let Some(hit) = find.here().cloned() else {
        return Vec::new();
    };
    let above = hit.row.saturating_sub(ABOVE_MATCH);
    ui.session.diff = above as f32 * line;
    let Some(at) = hit.line.map(|line| Caret::new(line, hit.range.start)) else {
        return Vec::new();
    };
    let end = Caret::new(at.line, hit.range.end);
    let holds = app
        .workspace
        .opened
        .as_ref()
        .is_some_and(|open| open.path == hit.path);
    if !holds {
        let open = workspace::Command::OpenFile {
            path: hit.path,
            at: Some(Selection {
                anchor: at,
                head: end,
            }),
        };
        return vec![Command::Workspace(open)];
    }
    [Edit::Move(Motion::To(at)), Edit::Extend(Motion::To(end))]
        .into_iter()
        .map(|edit| Command::Workspace(workspace::Command::Edit(edit)))
        .collect()
}

/// The search bar open or shut, its query spent either way. Only one thing takes
/// what is typed, so the commit box gives the keyboard up.
fn searching(ui: &mut Ui, on: bool) {
    ui.session.searching = on;
    ui.session.query.clear();
    if on {
        ui.session.composing = false;
    }
}

/// What a keystroke asks of the search bar: a path to narrow the list to, or the
/// first file it left.
fn in_query(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    match key {
        Key::Escape => searching(ui, false),
        Key::Enter => {
            let first = crate::views::session::components::files::narrowed(app, ui)
                .first()
                .map(|file| file.path.clone());
            searching(ui, false);
            return match first {
                Some(path) => vec![Command::Workspace(workspace::Command::OpenFile {
                    path,
                    at: None,
                })],
                None => Vec::new(),
            };
        }
        key => {
            typing(key, mods, &mut ui.session.query);
        }
    }
    Vec::new()
}

/// The open buffer takes the keystroke: a motion, a change, or a save.
fn in_file(key: Key, mods: Modifiers, app: &AppState) -> Vec<Command> {
    if app.workspace.opened.is_none() {
        return Vec::new();
    }
    if mods.ctrl {
        return with_ctrl(key).into_iter().collect();
    }
    let edit = match key {
        Key::Char(c) if !mods.alt => Edit::Insert(c.to_string()),
        Key::Enter => Edit::Newline,
        Key::Tab => Edit::Indent,
        Key::Backspace => Edit::Backspace,
        Key::Delete => Edit::Delete,
        _ => return moved(key, mods).into_iter().collect(),
    };
    vec![Command::Workspace(workspace::Command::Edit(edit))]
}

/// A motion, extending what the caret holds while shift is down.
fn moved(key: Key, mods: Modifiers) -> Option<Command> {
    let motion = match key {
        Key::Left => Motion::Left,
        Key::Right => Motion::Right,
        Key::Up => Motion::Up,
        Key::Down => Motion::Down,
        Key::Home => Motion::LineStart,
        Key::End => Motion::LineEnd,
        _ => return None,
    };
    let edit = match mods.shift {
        true => Edit::Extend(motion),
        false => Edit::Move(motion),
    };
    Some(Command::Workspace(workspace::Command::Edit(edit)))
}

/// What the control key asks of the buffer.
fn with_ctrl(key: Key) -> Option<Command> {
    let edit = match key {
        Key::Char('s' | 'S') => return Some(Command::Workspace(workspace::Command::SaveFile)),
        Key::Char('z' | 'Z') => Edit::Undo,
        Key::Char('y' | 'Y') => Edit::Redo,
        Key::Char('a' | 'A') => Edit::SelectAll,
        Key::Char('c' | 'C') => return Some(Command::Workspace(workspace::Command::Copy)),
        Key::Char('x' | 'X') => return Some(Command::Workspace(workspace::Command::Cut)),
        Key::Char('v' | 'V') => return Some(Command::Workspace(workspace::Command::Paste)),
        _ => return None,
    };
    Some(Command::Workspace(workspace::Command::Edit(edit)))
}

/// The list, or the commit message once the box has been clicked.
fn in_sidebar(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if !ui.session.composing {
        return in_list(key, ui, app);
    }
    if key == Key::Escape {
        ui.session.composing = false;
        return Vec::new();
    }
    if mods.ctrl && key == Key::Enter {
        ui.session.composing = false;
        return vec![Command::Workspace(workspace::Command::Commit)];
    }
    in_message(key, mods)
}

/// What a keystroke asks of the message.
fn in_message(key: Key, mods: Modifiers) -> Vec<Command> {
    if mods.ctrl {
        return Vec::new();
    }
    let edit = match key {
        Key::Char(c) if !mods.alt => Edit::Insert(c.to_string()),
        Key::Enter => Edit::Newline,
        Key::Backspace => Edit::Backspace,
        Key::Delete => Edit::Delete,
        Key::Left => Edit::Move(Motion::Left),
        Key::Right => Edit::Move(Motion::Right),
        Key::Up => Edit::Move(Motion::Up),
        Key::Down => Edit::Move(Motion::Down),
        Key::Home => Edit::Move(Motion::LineStart),
        Key::End => Edit::Move(Motion::LineEnd),
        _ => return Vec::new(),
    };
    vec![Command::Workspace(workspace::Command::Message(edit))]
}

/// Up and down open the file above or below in the list.
fn in_list(key: Key, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let files = crate::views::session::changed(app);
    let at = files
        .iter()
        .position(|file| Some(&file.path) == app.workspace.opened.as_ref().map(|o| &o.path));
    let next = match (key, at) {
        (Key::Up, Some(at)) => at.saturating_sub(1),
        (Key::Down, Some(at)) => (at + 1).min(files.len().saturating_sub(1)),
        (Key::Up | Key::Down, None) => 0,
        (Key::Enter, _) => {
            ui.focus = Focus::Workspace;
            return Vec::new();
        }
        _ => return Vec::new(),
    };
    let Some(file) = files.get(next) else {
        return Vec::new();
    };
    vec![Command::Workspace(workspace::Command::OpenFile {
        at: None,
        path: file.path.clone(),
    })]
}

/// Up and down move along the opened sessions.
fn in_rail(key: Key, app: &AppState) -> Vec<Command> {
    let at = app
        .session
        .open
        .iter()
        .position(|open| Some(&open.session.id) == app.session.selected.as_ref());
    let next = match (key, at) {
        (Key::Up, Some(at)) => at.saturating_sub(1),
        (Key::Down, Some(at)) => (at + 1).min(app.session.open.len().saturating_sub(1)),
        (Key::Up | Key::Down, None) => 0,
        _ => return Vec::new(),
    };
    let Some(open) = app.session.open.get(next) else {
        return Vec::new();
    };
    vec![Command::Session(session::Command::Select {
        session: open.session.id.clone(),
    })]
}

/// Groove's own shortcuts.
fn chord(key: Key, ui: &mut Ui, app: &AppState) -> Option<Command> {
    match key {
        Key::Char('p' | 'P') => {
            if ui.palette.take().is_some() {
                return None;
            }
            ui.palette = Some(Palette::default());
            Some(Command::Session(session::Command::ListRepos))
        }
        Key::Char('n' | 'N') => Some(Command::Session(session::Command::OpenExplorer {
            title: None,
        })),
        Key::Char('b' | 'B') => {
            ui.session.folded = !ui.session.folded;
            None
        }
        Key::Char('r' | 'R') => Some(Command::Workspace(workspace::Command::Load)),
        Key::Left | Key::Right => {
            ui.focus = ui.focus.beside(key == Key::Right);
            None
        }
        Key::Char('w' | 'W') => {
            let session = app.session.selected.clone()?;
            Some(Command::Session(session::Command::Close { session }))
        }
        _ => None,
    }
}

fn to_agent(key: Key, mods: Modifiers, app: &AppState) -> Option<Command> {
    let session = app.session.selected.clone()?;
    let bytes = encode(key, mods)?;
    Some(Command::Agent(agent::Command::Send { session, bytes }))
}

/// The bytes a terminal sends for a key. `None` for a key a terminal has no word for.
pub fn encode(key: Key, mods: Modifiers) -> Option<Vec<u8>> {
    let bytes = match key {
        Key::Char(c) if mods.ctrl => vec![control(c)?],
        Key::Char(c) if mods.alt => {
            let mut b = vec![0x1b];
            b.extend(c.to_string().into_bytes());
            b
        }
        Key::Char(c) => c.to_string().into_bytes(),
        Key::Enter => b"\r".to_vec(),
        Key::Escape => b"\x1b".to_vec(),
        Key::Tab if mods.shift => b"\x1b[Z".to_vec(),
        Key::Tab => b"\t".to_vec(),
        Key::Backspace => b"\x7f".to_vec(),
        Key::Delete => b"\x1b[3~".to_vec(),
        Key::Up => b"\x1b[A".to_vec(),
        Key::Down => b"\x1b[B".to_vec(),
        Key::Right => b"\x1b[C".to_vec(),
        Key::Left => b"\x1b[D".to_vec(),
        Key::Home => b"\x1b[H".to_vec(),
        Key::End => b"\x1b[F".to_vec(),
        Key::PageUp => b"\x1b[5~".to_vec(),
        Key::PageDown => b"\x1b[6~".to_vec(),
    };
    Some(bytes)
}

/// `ctrl+a` is 0x01 … `ctrl+z` is 0x1a; `ctrl+[` `\` `]` `^` `_` follow.
fn control(c: char) -> Option<u8> {
    let c = c.to_ascii_lowercase();
    match c {
        'a'..='z' => Some(c as u8 - b'a' + 1),
        '[' => Some(0x1b),
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '-' => Some(0x1f),
        ' ' | '2' | '@' => Some(0x00),
        _ => None,
    }
}
