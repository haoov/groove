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

const fn pane(
    action: Action,
    (id, label): (&'static str, &'static str),
    chords: &'static [&'static str],
) -> Spec {
    spec(action, (id, label, "Panes"), Ring::App, chords)
}

use Action as A;
use Ring::{App, Code, Terminal};

pub const TABLE: [Spec; 43] = [
    spec(
        A::Palette,
        ("palette.open", "palette", "General"),
        App,
        &["alt+k"],
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
        &["alt+h"],
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
        &["alt+shift+w"],
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
    spec(
        A::FocusLeft,
        ("pane.left", "pane to the left", "Panes"),
        App,
        &["alt+shift+left"],
    ),
    spec(
        A::FocusRight,
        ("pane.right", "pane to the right", "Panes"),
        App,
        &["alt+shift+right"],
    ),
    pane(A::FocusRail, ("pane.rail", "the rail"), &["alt+1"]),
    pane(A::FocusAgent, ("pane.agent", "the agent"), &["alt+2"]),
    pane(
        A::FocusWorkspace,
        ("pane.workspace", "the workspace"),
        &["alt+3"],
    ),
    pane(A::FocusSidebar, ("pane.sidebar", "the sidebar"), &["alt+4"]),
    pane(
        A::FocusCommit,
        ("pane.commit", "the commit box"),
        &["alt+c"],
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
    spec(
        A::FoldSidebar,
        ("sidebar.fold", "fold the sidebar", "Panes"),
        App,
        &["alt+shift+b"],
    ),
    spec(
        A::Overview,
        ("tab.overview", "overview tab", "Panes"),
        App,
        &["alt+shift+1"],
    ),
    spec(
        A::Diff,
        ("tab.diff", "diff tab", "Panes"),
        App,
        &["alt+shift+2"],
    ),
    spec(
        A::Files,
        ("tab.files", "files tab", "Panes"),
        App,
        &["alt+shift+3"],
    ),
    spec(
        A::Resources,
        ("tab.resources", "kubernetes tab", "Panes"),
        App,
        &["alt+shift+4"],
    ),
    spec(
        A::Terminals,
        ("terminals.toggle", "terminals", "Panes"),
        App,
        &["alt+'"],
    ),
    spec(
        A::NewTerminal,
        ("terminals.new", "new terminal", "Panes"),
        App,
        &["alt+shift+'"],
    ),
    spec(
        A::Reload,
        ("workspace.load", "reload the workspace", "Panes"),
        App,
        &["alt+shift+r"],
    ),
    spec(A::Save, ("editor.save", "save", "Code"), Code, &["ctrl+s"]),
    spec(A::Undo, ("editor.undo", "undo", "Code"), Code, &["ctrl+z"]),
    spec(
        A::Redo,
        ("editor.redo", "redo", "Code"),
        Code,
        &["ctrl+shift+z", "ctrl+y"],
    ),
    spec(
        A::SelectAll,
        ("editor.select_all", "select all", "Code"),
        Code,
        &["ctrl+a"],
    ),
    spec(A::Copy, ("editor.copy", "copy", "Code"), Code, &["ctrl+c"]),
    spec(A::Cut, ("editor.cut", "cut", "Code"), Code, &["ctrl+x"]),
    spec(
        A::Paste,
        ("editor.paste", "paste", "Code"),
        Code,
        &["ctrl+v"],
    ),
    spec(
        A::Find,
        ("find.open", "find in the file or the list", "Code"),
        Code,
        &["ctrl+f"],
    ),
    spec(
        A::SelectNamespace,
        ("resources.select_namespace", "select a namespace", "Code"),
        Code,
        &["ctrl+shift+n"],
    ),
    spec(
        A::FindNext,
        ("find.next", "next match", "Code"),
        Code,
        &["ctrl+n"],
    ),
    spec(
        A::FindPrevious,
        ("find.previous", "previous match", "Code"),
        Code,
        &["ctrl+p"],
    ),
    spec(
        A::OpenPath,
        (
            "files.open_path",
            "open a file by path, or search the kinds",
            "Code",
        ),
        Code,
        &["ctrl+shift+p"],
    ),
    spec(
        A::SearchFiles,
        ("files.search", "search in files", "Code"),
        Code,
        &["ctrl+shift+f"],
    ),
    spec(
        A::Commit,
        ("commit", "commit", "Code"),
        Code,
        &["ctrl+enter"],
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
