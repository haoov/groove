//! The actions, in the order Settings lists them, each with its defaults.

use super::Ring;

pub use super::action::Action;

/// One action as the keymap knows it.
pub struct Spec {
    pub action: Action,
    /// What the config file keys its chords by.
    pub id: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub ring: Ring,
    pub defaults: &'static [&'static str],
}

const fn spec(
    action: Action,
    (id, label, group): (&'static str, &'static str, &'static str),
    ring: Ring,
    defaults: &'static [&'static str],
) -> Spec {
    Spec {
        action,
        id,
        label,
        group,
        ring,
        defaults,
    }
}

/// A region the keys go to, in the Panes group.
/// An open file's or object's tab, while the workspace has the keys.
const fn opened(
    action: Action,
    (id, label): (&'static str, &'static str),
    chords: &'static [&'static str],
) -> Spec {
    spec(action, (id, label, "Panes"), Ring::Code, chords)
}

/// An action on the focused pane's content, in the Code group.
const fn code(
    action: Action,
    (id, label): (&'static str, &'static str),
    chords: &'static [&'static str],
) -> Spec {
    spec(action, (id, label, "Code"), Ring::Code, chords)
}

const fn pane(
    action: Action,
    (id, label): (&'static str, &'static str),
    chords: &'static [&'static str],
) -> Spec {
    spec(action, (id, label, "Panes"), Ring::App, chords)
}

use Action as A;
use Ring::{App, Code, List, Terminal};

pub const TABLE: [Spec; 56] = [
    spec(
        A::Palette,
        ("palette.open", "palette", "General"),
        App,
        &["alt+shift+;"],
    ),
    spec(
        A::Settings,
        ("settings.open", "settings", "General"),
        App,
        &["alt+,"],
    ),
    spec(
        A::Board,
        ("board.toggle", "board, or back", "General"),
        App,
        &["alt+shift+b"],
    ),
    spec(
        A::FindFile,
        ("palette.files", "find a file", "General"),
        App,
        &["alt+f"],
    ),
    spec(
        A::NewExplorer,
        ("session.open_explorer", "new explorer", "Sessions"),
        App,
        &["alt+shift+n"],
    ),
    spec(
        A::CloseSession,
        ("session.close", "close session", "Sessions"),
        App,
        &["alt+shift+q"],
    ),
    spec(
        A::NextSession,
        ("session.next", "next session", "Sessions"),
        App,
        &["alt+shift+down"],
    ),
    spec(
        A::PreviousSession,
        ("session.previous", "previous session", "Sessions"),
        App,
        &["alt+shift+up"],
    ),
    pane(
        A::FocusLeft,
        ("pane.left", "pane to the left"),
        &["alt+shift+left"],
    ),
    pane(
        A::FocusRight,
        ("pane.right", "pane to the right"),
        &["alt+shift+right"],
    ),
    pane(A::FocusRail, ("pane.rail", "the rail"), &["alt+r"]),
    pane(A::BoardLive, ("board.live", "board live"), &["alt+1"]),
    pane(A::BoardNext, ("board.next", "board up next"), &["alt+2"]),
    pane(A::BoardReview, ("board.review", "board review"), &["alt+3"]),
    pane(A::FocusAgent, ("pane.agent", "the agent"), &["alt+a"]),
    pane(
        A::FocusWorkspace,
        ("pane.workspace", "the workspace: open, focus, fold"),
        &["alt+o"],
    ),
    pane(
        A::FocusSidebar,
        ("pane.sidebar", "the sidebar: open, focus, fold"),
        &["alt+e"],
    ),
    pane(
        A::ResizeLeft,
        ("pane.narrower", "boundary left"),
        &["ctrl+alt+h"],
    ),
    pane(
        A::ResizeRight,
        ("pane.wider", "boundary right"),
        &["ctrl+alt+l"],
    ),
    pane(A::ResizeUp, ("pane.taller", "boundary up"), &["ctrl+alt+k"]),
    pane(
        A::ResizeDown,
        ("pane.shorter", "boundary down"),
        &["ctrl+alt+j"],
    ),
    pane(A::NextTab, ("tab.next", "next tab"), &["alt+]"]),
    pane(A::PreviousTab, ("tab.previous", "previous tab"), &["alt+["]),
    opened(
        A::CloseOpen,
        ("open.close", "close the open tab"),
        &["ctrl+w"],
    ),
    opened(
        A::NextOpen,
        ("open.next", "next open tab"),
        &["ctrl+tab", "ctrl+pagedown"],
    ),
    opened(
        A::PreviousOpen,
        ("open.previous", "previous open tab"),
        &["ctrl+shift+tab", "ctrl+pageup"],
    ),
    pane(
        A::Overview,
        ("tab.overview", "overview tab"),
        &["alt+shift+1"],
    ),
    pane(A::Diff, ("tab.diff", "diff tab"), &["alt+shift+2"]),
    pane(A::Files, ("tab.files", "files tab"), &["alt+shift+3"]),
    pane(
        A::Resources,
        ("tab.resources", "kubernetes tab"),
        &["alt+shift+4"],
    ),
    pane(
        A::SelectNamespace,
        ("resources.select_namespace", "pick the namespaces"),
        &["alt+n"],
    ),
    pane(
        A::SelectContext,
        ("resources.select_context", "pick the contexts"),
        &["alt+c"],
    ),
    pane(
        A::SelectKind,
        ("resources.select_kind", "pick a kind"),
        &["alt+k"],
    ),
    pane(
        A::SelectWorktree,
        ("session.select_worktree", "pick a worktree"),
        &["alt+w"],
    ),
    pane(
        A::SelectSession,
        ("session.select", "pick a session"),
        &["alt+s"],
    ),
    pane(A::Terminals, ("terminals.toggle", "terminals"), &["alt+'"]),
    pane(
        A::NewTerminal,
        ("terminals.new", "new terminal"),
        &["alt+shift+'"],
    ),
    pane(
        A::Reload,
        ("workspace.load", "reload the workspace"),
        &["alt+shift+r"],
    ),
    spec(
        A::ListDown,
        ("list.down", "down a list", "Lists"),
        List,
        &["ctrl+j"],
    ),
    spec(
        A::ListUp,
        ("list.up", "up a list", "Lists"),
        List,
        &["ctrl+k"],
    ),
    code(A::Save, ("editor.save", "save"), &["ctrl+s"]),
    code(
        A::Hover,
        ("editor.hover", "what the servers say here"),
        &["ctrl+k"],
    ),
    spec(
        A::Complete,
        (
            "editor.complete",
            "what the servers offer to type here",
            "Code",
        ),
        Code,
        &["ctrl+space"],
    ),
    code(A::Undo, ("editor.undo", "undo"), &["ctrl+z"]),
    code(
        A::Redo,
        ("editor.redo", "redo"),
        &["ctrl+shift+z", "ctrl+y"],
    ),
    code(
        A::SelectAll,
        ("editor.select_all", "select all"),
        &["ctrl+a"],
    ),
    code(A::Copy, ("editor.copy", "copy"), &["ctrl+c"]),
    code(A::Cut, ("editor.cut", "cut"), &["ctrl+x"]),
    code(A::Paste, ("editor.paste", "paste"), &["ctrl+v"]),
    code(
        A::Find,
        ("find.open", "find in the file or the list"),
        &["ctrl+f"],
    ),
    code(A::FindNext, ("find.next", "next match"), &["enter"]),
    code(
        A::FindPrevious,
        ("find.previous", "previous match"),
        &["shift+enter"],
    ),
    code(
        A::OpenPath,
        ("files.open_path", "open a file by path"),
        &["ctrl+p"],
    ),
    code(
        A::SearchFiles,
        ("files.search", "search in files"),
        &["ctrl+shift+f"],
    ),
    spec(
        A::TerminalCopy,
        ("terminal.copy", "copy", "Terminals"),
        Terminal,
        &["ctrl+shift+c"],
    ),
    spec(
        A::TerminalPaste,
        ("terminal.paste", "paste", "Terminals"),
        Terminal,
        &["ctrl+shift+v"],
    ),
];
