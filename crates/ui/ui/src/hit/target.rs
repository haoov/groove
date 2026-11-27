//! What a point of the frame stands for, as the click that lands on it reads it.

use groove_types::{SessionId, WorktreeId};

use groove_types::DiffView;

use super::{Answering, NoteButton, Picks};
use crate::layout::Edge;

/// A thing on screen the pointer can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A rail row.
    Session(SessionId),
    /// What the change is read against.
    Mode(groove_types::DiffMode),
    /// One of the header's pickers.
    Picker(Picks),
    /// An overview row.
    Worktree(WorktreeId),
    /// The overview's ground, under its rows.
    Overview,
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

    /// Settings' own bar's back.
    SettingsBack,
    /// A section of the list, and the search over them.
    SettingsSection(crate::views::settings::Section),
    SettingsSearch,
    /// Setup's environment check run again, the `claude` sign-in opened, and ended.
    SettingsCheck,
    SettingsLogin,
    SettingsLoginEnd,
    /// The alerts' `inc auth login`.
    SettingsIncLogin,
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
    /// The shared repo: its fields opened, then asked gone, then the answer that confirms it.
    SettingsShare,
    SettingsUnshare,
    SettingsUnshareSure,
    /// A skill switched to `on`; one of the user's own asked deleted, then the answer.
    SkillSwitch(String, bool),
    SkillDelete(String),
    SkillDeleteSure(String),
    SkillDeleteKeep,
    /// A routine switched on or off, or run; one trigger of it.
    RoutineOn(String),
    RoutineRun(String),
    RoutineOff(String),
    TriggerSwitch(String, groove_types::Trigger, bool),
    /// An action's chord waited for, or put back on its defaults.
    SettingsBind(crate::keymap::Action),
    SettingsUnbind(crate::keymap::Action),
    /// What opens the menu of one slot of a source's mapping.
    SettingsPick(groove_types::ProviderId, crate::views::settings::rows::Slot),
    /// A kubeconfig context added or removed, or one of its switches set.
    ClusterAdd(String),
    ClusterRemove(String),
    ClusterSet(String, groove_types::ClusterChange),
    /// A context's dot, which opens the hues it can take.
    ClusterHue(String),
    /// A kind in the Kubernetes sidebar, the list's search, and one row of the list.
    ResourceKind(groove_types::KubeKind),
    ResourceSearch,
    /// An Application's find bar, which a click types in.
    ArgoSearch,
    ResourceGroup(groove_types::KindHeading),
    ResourceRow(String),
    /// A column of the list, by its label: its header sorts by it, its edge drags its width.
    ResourceSort(String),
    ResourceEdge(String),
    /// An open object's row in the sidebar, the mark that closes it, and its view.
    ResourceTab(usize),
    ResourceClose(usize),
    ResourceView(crate::views::session::resources::View),
    /// The band's pin toggle, a pinned list's row, and its cross.
    ResourcePin,
    PinnedList(Box<groove_types::Pin>),
    Unpin(Box<groove_types::Pin>),
    /// An object's YAML made writable, or read-only again, its draft thrown away; and the draft applied.
    YamlWrite(bool),
    YamlApply,
    /// An Application's resources view: a filter picked.
    ArgoFilter(crate::views::session::resources::Filter),
    /// A group or a row of an object's tab, folded or unfolded, and whether it was drawn shut.
    ResourceFold(String, bool),
    /// The logs view's controls: the containers read, the run, the range picker, following.
    LogSource(groove_types::LogSource),
    LogPrevious(bool),
    LogRange,
    LogFollow,
    /// A container's cpu, or with `true` its memory, which shows its request and limit under the pointer.
    ContainerUsage(String, bool),
    /// A section of the described view folded, and another object to open.
    ResourceSection(crate::views::session::resources::Section),
    ResourceOpen(Box<crate::views::session::resources::Link>),
    /// A value of an object's tab, which a click copies; a ctrl+click opens its object.
    ResourceCopy(String, Option<Box<crate::views::session::resources::Opens>>),
    /// A context's line, which only lights under the pointer.
    ClusterRow(String),
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
    /// A directory of the changed list, which a click folds over its files or opens.
    Group(String),
    /// Which of the sidebar's lists is up.
    Pane(crate::views::session::Pane),
    /// Which notes the sidebar's list shows.
    Showing(crate::views::session::Showing),
    /// One note of the sidebar's list, which a click opens the line of.
    NoteAt(usize),
    /// One commit of the sidebar's list, which a click shows the change of.
    Commit(String),
    /// The blame after a line, which a click shows the commit of.
    Blamed(String),
    /// What leaves the commit and shows the working tree again.
    Working,
    /// The feed's own heading, which folds it away.
    Feed,
    /// The sessions' window's header, which folds it to itself.
    SessionsFold,
    /// A section's heading on the rail, which folds it to itself.
    SectionFold(groove_types::Section),
    /// A line of the feed, and the session it belongs to.
    FeedLine(SessionId),
    /// A folded run of the feed, opened or folded again.
    FeedRun(crate::views::rail::RunKey),
    /// The write the agent asked for, taken or refused.
    Approve(groove_types::ApprovalId),
    /// The sheet itself, which keeps a click from what it covers.
    Sheet,
    Refuse(groove_types::ApprovalId),
    /// The feed narrowed to the session in hand, or widened to every session.
    FeedScope(bool),
    /// One MR of the review column, by its project and its number.
    Review(String, u64),
    /// A column heading of the review table, which orders it.
    SortReview(crate::views::board::review::By),
    /// An MR's number, a link to its page.
    MrPage(String),
    /// What opens the task's own page at its source.
    TaskPage(String),
    /// A link in prose.
    Link(String),
    /// The sessions' window's `+`, which opens a new explorer.
    NewExplorer,
    /// A boundary between two columns.
    Split(Edge),
    /// A file's row in the sidebar.
    File(String),
    /// The two answers to what is asked before a change is thrown away.
    Discard,
    Keep,
    /// The count a row's question sets, one step down or up.
    Step(i32),
    /// A row of the menu the right button opens.
    MenuRow(usize),
    /// A scope panel: its body, a line of it, and a held line's ×.
    ScopePanel,
    ScopeLine(usize),
    ScopeDetach(usize),
    /// The open file's rows.
    Code,
    /// The lines standing above them.
    Pinned,
    /// The column holding the whole change.
    Map,
    /// What marks a file read, at the end of its head row.
    Read(String),
    /// The row a file starts on, which folds it.
    /// The caret that folds the block a line of the open file opens.
    Block(usize),
    /// The lines a gap hides, and which end gives them up.
    Gap {
        row: usize,
        way: groove_controllers::workspace::Way,
    },
    /// The bar over the rows while a search of them is live.
    Finding,
    /// One of the three views of the open file.
    View(DiffView),
    /// The agent's pane.
    Agent,
    /// One tab of the session's bar.
    ModeTab(crate::views::session::Mode),
    /// The head of the session's sidebar, which folds it.
    AsideFold,
    /// A list's scrollbar, its thumb dragged along it.
    Thumb(super::Scroller),
    /// The head of a card on the modes Overview, which folds it, by key.
    CardFold(String),
    /// The modes Diff's list of changed files, under its rows.
    Listing,
    /// The conversation's column, the file an edit wrote at its hunk's first line, and the prompt.
    Chat,
    Edited {
        path: String,
        line: u32,
    },
    Prompt,
    /// A block that opens and folds: steps by their key, a sent agent by its id.
    Commands(String),
    /// An answer to what the headless agent asks.
    Answer(Answering),
    /// The line of usage under the prompt, which opens the models' menu.
    ModelButton,
    /// One of the commands `/` offers, by its place in the offers.
    Offered(usize),
    /// An agent's row in the sidebar: its card, opened and in sight.
    Subagent(String),
    /// One button of a thread's own row, by the thread's id.
    Note(String, NoteButton),
    /// What the composer of the reply being typed offers.
    Compose(super::Composing),
}
