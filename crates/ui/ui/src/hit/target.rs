//! What a point of the frame stands for, as the click that lands on it reads it.

use groove_types::{SessionId, WorktreeId};

use groove_types::DiffView;

use super::{Cursor, NoteButton, Picks};
use crate::layout::Edge;
use crate::views::session::{Tab, Term};

/// A thing on screen the pointer can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A rail row.
    Session(SessionId),
    /// What the change is read against.
    Mode(groove_types::DiffMode),
    /// A tab of the workspace.
    Tab(Tab),
    /// One of the header's two pickers.
    Picker(Picks),
    /// An overview row.
    Worktree(WorktreeId),
    /// A palette row, by its place in the list.
    PaletteRow(usize),
    /// The palette's box; a click on it does nothing.
    Palette,
    /// The rail's own first row, which opens the board.
    Board,
    /// A task on the board, which a click opens the session for.
    Task(String),
    /// A task's place in the plan, which a drag moves.
    Place(groove_types::ExternalId),
    /// What hands the source the hours the clock measured.
    LogHours(groove_types::ExternalId),
    /// The timeline's own bar, which folds it away.
    Timeline,
    /// The rail's footer row, which opens Settings, and its own bar's back.
    SettingsOpen,
    SettingsBack,
    /// A section of the list, and the search over them.
    SettingsSection(crate::views::settings::Section),
    SettingsSearch,
    /// Setup's environment check run again, the `claude` sign-in opened, and ended.
    SettingsCheck,
    SettingsLogin,
    SettingsLoginEnd,
    /// The sign-in's terminal, which takes the keys while it runs.
    Login,
    /// A task source: its fields opened to turn it on, one of them given the keys, sent.
    SettingsTurnOn(groove_types::ProviderId),
    SettingsDraftField(usize),
    SettingsConnect,
    SettingsDraftCancel,
    /// A source asked off, then the answer that confirms it.
    SettingsTurnOff(groove_types::ProviderId),
    SettingsTurnOffSure(groove_types::ProviderId),
    /// An action's chord waited for, or put back on its defaults.
    SettingsBind(crate::keymap::Action),
    SettingsUnbind(crate::keymap::Action),
    /// What opens the menu of one slot of a source's mapping.
    SettingsPick(groove_types::ProviderId, crate::views::settings::rows::Slot),
    /// A control of a Settings row: the preference a click on it sets.
    SetPreference(groove_controllers::config_service::Preference),
    /// What folds the manual section away, or opens it.
    ShellFold,
    /// What opens a terminal in a tab of its own.
    ShellNew,
    /// What opens one beside the terminals of the tab that is up.
    ShellSplit,
    /// One tab, and the cross that ends every terminal in it.
    ShellTab(u64),
    ShellCloseTab(u64),
    /// One terminal's grid, which a click gives the keyboard, and the cross that ends it alone.
    Shell(u64),
    ShellClose(u64),
    /// An open file's tab, which a click makes the active one.
    OpenTab(String),
    /// What closes that tab.
    CloseTab(String),
    /// A directory of the explorer, which a click opens or shuts.
    Dir(String),
    /// Which of the sidebar's lists is up.
    Pane(crate::views::session::Pane),
    /// One note of the sidebar's list, which a click opens the line of.
    NoteAt(usize),
    /// One commit of the sidebar's list, which a click shows the change of.
    Commit(String),
    /// What leaves the commit and shows the working tree again.
    Working,
    /// The feed's own heading, which folds it away.
    Feed,
    /// A line of the feed, and the session it belongs to.
    FeedLine(SessionId),
    /// The write the agent asked for, taken or refused.
    Approve(groove_types::ApprovalId),
    /// The sheet that shows a write before it is decided.
    Examine(groove_types::ApprovalId),
    /// The sheet itself, which keeps a click from what it covers.
    Sheet,
    /// The switch that lets a session's writes through without asking.
    AutoApprove(groove_types::SessionId),
    Refuse(groove_types::ApprovalId),
    /// The agent's own row: what it can be sent, and its reload.
    Skills(SessionId),
    Reload(SessionId),
    /// Which sessions the feed shows.
    FeedScope,
    /// One MR of the review column, by its project and its number.
    Review(String, u64),
    /// What finishes the task a session works, and what opens its other actions.
    Finish(groove_types::SessionId),
    /// What reads the selected worktree's MR again.
    Refresh,
    /// An MR's number, a link to its page.
    MrPage(String),
    /// What opens the task's own page at its source.
    TaskPage(String),
    TaskActions(groove_types::SessionId),
    /// A link in prose.
    Link(String),
    /// One task's bar on the timeline, which names it under the pointer.
    Bar(String),
    /// The board's filter, and one row it offers.
    Filter,
    Offer(usize),
    /// What starts a task, at the right of the board's header.
    AddTask,
    /// A boundary between two columns.
    Split(Edge),
    /// What folds the sidebar away.
    Fold,
    /// A file's row in the sidebar.
    File(String),
    /// What a row offers while the pointer is on it.
    Stage(String),
    Unstage(String),
    /// The two answers to what is asked before a change is thrown away.
    Discard,
    Keep,
    /// What the commit box offers beyond committing.
    Actions,
    /// A row of the menu the right button opens.
    MenuRow(usize),
    /// The commit message, and what the box does now.
    Message,
    Do,
    /// The open file's rows.
    Code,
    /// The lines standing above them.
    Pinned,
    /// The column holding the whole change.
    Map,
    /// What marks a file read, at the end of its head row.
    Read(String),
    /// The row a file starts on, which folds it.
    Head(String),
    /// The lines a gap hides, and which end gives them up.
    Gap {
        row: usize,
        way: groove_controllers::workspace::Way,
    },
    /// One line a search across the worktree found, by its place in the list.
    Found(usize),
    /// The row naming a file the search found lines in, which folds them.
    FoundIn(String),
    /// One term of the sidebar's search bar.
    Term(Term),
    /// The bar over the rows while a search of them is live.
    Finding,
    /// One of the three views of the open file.
    View(DiffView),
    /// The agent's pane.
    Agent,
    /// One button of a note's own row.
    Note(groove_types::NoteOrigin, NoteButton),
}

impl Target {
    /// What the pointer says over it: a row is a pointer, and these are not.
    pub(super) fn cursor(&self) -> Cursor {
        match self {
            Target::Term(_) | Target::Finding | Target::Filter | Target::Message | Target::Code => {
                Cursor::Text
            }
            Target::Agent
            | Target::Shell(_)
            | Target::Pinned
            | Target::Bar(_)
            | Target::Palette => Cursor::Default,
            Target::Map | Target::Place(_) => Cursor::RowResize,
            Target::Split(edge) => match edge.upright() {
                true => Cursor::ColResize,
                false => Cursor::RowResize,
            },
            _ => Cursor::Pointer,
        }
    }
}
