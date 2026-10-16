//! The palette's every-day actions, beyond the session's own: where to go, the diff, git,
//! the MR, the agent, the notes, the task and the look.

use groove_controllers::config_service::Preference;
use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command, agent, config, task, workspace};
use groove_types::{ApprovalId, DiffView, SessionKind, ThemeName};

use super::{Entry, Run};
use crate::keymap::Action;
use crate::views::session::Showing;

/// What a palette row asks of the window rather than of a service.
#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    /// What the keymap action does from its chord.
    Key(Action),
    View(DiffView),
    /// A note on the caret's line of the open file.
    Note,
    Notes(Showing),
    /// The agent's ask, shown in its sheet.
    Examine(ApprovalId),
    /// A file of the selected worktree, opened on the Files tab.
    Open(String),
    /// Every change of the worktree thrown away, once confirmed.
    Discard,
}

impl Act {
    pub fn id(&self) -> &'static str {
        match self {
            Act::Key(action) => crate::keymap::id_of(*action),
            Act::View(DiffView::Inline) => "diff.inline",
            Act::View(DiffView::Split) => "diff.split",
            Act::Note => "notes.add",
            Act::Notes(_) => "notes.show",
            Act::Examine(_) => "agent.review",
            Act::Open(_) => "workspace.open_file",
            Act::Discard => "workspace.discard_all",
        }
    }
}

fn act(group: &'static str, label: impl Into<String>, act: Act) -> Entry {
    Entry {
        group,
        label: label.into(),
        run: Run::Act(act),
    }
}

fn command(group: &'static str, label: impl Into<String>, command: Command) -> Entry {
    Entry {
        group,
        label: label.into(),
        run: Run::Command(command),
    }
}

/// Every action that applies now, group after group.
pub(super) fn actions(app: &AppState) -> Vec<Entry> {
    let mut out = vec![act("Go", "Board", Act::Key(Action::Board))];
    let Some(open) = app.session.selected() else {
        out.extend(themes(app));
        return out;
    };
    out.extend(going(open));
    out.extend(diffing(app));
    out.extend(landing(app, open));
    out.extend(agent(app, open));
    out.extend(noting(app));
    out.extend(tasked(app, open));
    out.extend(super::skills::entries(app, open));
    out.extend(themes(app));
    out
}

/// The session's tabs, its sidebar and terminals, the worktree read again.
fn going(open: &Open) -> Vec<Entry> {
    let mut out = vec![
        act("Go", "Overview", Act::Key(Action::Overview)),
        act("Go", "Diff", Act::Key(Action::Diff)),
        act("Go", "Files", Act::Key(Action::Files)),
    ];
    if open.holds_clusters() {
        out.push(act("Go", "Kubernetes", Act::Key(Action::Resources)));
    }
    out.extend([
        act(
            "Go",
            "Show or hide the sidebar",
            Act::Key(Action::FoldSidebar),
        ),
        act("Go", "Terminals", Act::Key(Action::Terminals)),
        act("Go", "Keys to the rail", Act::Key(Action::FocusRail)),
        act("Go", "Keys to the agent", Act::Key(Action::FocusAgent)),
        act(
            "Go",
            "Keys to the workspace",
            Act::Key(Action::FocusWorkspace),
        ),
        act("Go", "Keys to the sidebar", Act::Key(Action::FocusSidebar)),
        act(
            "Go",
            "Keys to the commit box",
            Act::Key(Action::FocusCommit),
        ),
        act("Go", "New terminal", Act::Key(Action::NewTerminal)),
        act("Go", "Reload the worktree", Act::Key(Action::Reload)),
    ]);
    out
}

/// The diff's two views, while a worktree is selected.
fn diffing(app: &AppState) -> Vec<Entry> {
    if app.session.selected_worktree().is_none() {
        return Vec::new();
    }
    vec![
        act("Diff", "Inline view", Act::View(DiffView::Inline)),
        act("Diff", "Split view", Act::View(DiffView::Split)),
    ]
}

/// Git on the selected worktree, then its MR: open it once it has one, else create it.
fn landing(app: &AppState, open: &Open) -> Vec<Entry> {
    let Some(worktree) = app.session.selected_worktree() else {
        return Vec::new();
    };
    let git = |label: &str, one: workspace::Command| command("Git", label, Command::Workspace(one));
    let mut out = vec![
        git("Commit what is staged", workspace::Command::Commit),
        git("Push", workspace::Command::Push),
        git("Pull", workspace::Command::Pull),
    ];
    out.extend(super::mr::entries(app, open, worktree));
    out.push(act("Git", "Discard every change", Act::Discard));
    out
}

/// The agent's first ask answered or shown, the agent started again, its writes let through.
fn agent(app: &AppState, open: &Open) -> Vec<Entry> {
    let session = open.session.id.clone();
    let mut out = Vec::new();
    let ask = app
        .agent
        .activity(&session)
        .and_then(|one| one.asks.first());
    if let Some(ask) = ask {
        let asks = crate::views::rail::asks_to(&ask.op);
        let approve = agent::Command::Approve { id: ask.id.clone() };
        out.push(command(
            "Agent",
            format!("Approve: {asks}"),
            Command::Agent(approve),
        ));
        out.push(act(
            "Agent",
            format!("Review: {asks}"),
            Act::Examine(ask.id.clone()),
        ));
    }
    let (cols, rows) = groove_controllers::session::FIRST_SIZE;
    let reload = agent::Command::Reload {
        session: session.clone(),
        cols,
        rows,
    };
    out.push(command("Agent", "Reload the agent", Command::Agent(reload)));
    let on = !open.state.auto_approve;
    let label = if on {
        "Turn auto-approve on"
    } else {
        "Turn auto-approve off"
    };
    let switch = agent::Command::AutoApprove { session, on };
    out.push(command("Agent", label, Command::Agent(switch)));
    out
}

/// A note where the caret stands, and the sidebar's notes list on what it shows.
fn noting(app: &AppState) -> Vec<Entry> {
    let mut out = Vec::new();
    if app.workspace.active().is_some() {
        out.push(act("Notes", "Add a note on the caret's line", Act::Note));
    }
    for showing in Showing::ALL {
        let label = format!("Show {} notes", showing.label());
        out.push(act("Notes", label, Act::Notes(showing)));
    }
    out
}

/// A task finished, or opened where it is filed.
fn tasked(app: &AppState, open: &Open) -> Vec<Entry> {
    if !matches!(open.session.kind, SessionKind::Task { .. }) {
        return Vec::new();
    }
    let finish = task::Command::Finish {
        session: open.session.id.clone(),
    };
    let mut out = vec![command("Task", "Finish the task", Command::Task(finish))];
    if let Some(url) = app
        .task
        .worked(&open.session)
        .and_then(|one| one.url.clone())
    {
        let browse = task::Command::Browse { url };
        out.push(command(
            "Task",
            "Open the task where it is filed",
            Command::Task(browse),
        ));
    }
    out
}

/// Each theme but the one on, then the code's wrap turned the other way.
fn themes(app: &AppState) -> Vec<Entry> {
    let on = app.config.theme();
    let wrap = app.config.wrap_code();
    let label = if wrap {
        "Stop wrapping long lines"
    } else {
        "Wrap long lines"
    };
    let flip = config::Command::SetPreference(Preference::WrapCode(!wrap));
    let wrapping = command("Look", label, Command::Config(flip));
    ThemeName::ALL
        .into_iter()
        .filter(|one| *one != on)
        .map(|one| {
            let set = config::Command::SetPreference(Preference::Theme(one));
            command(
                "Look",
                format!("Theme: {}", one.label()),
                Command::Config(set),
            )
        })
        .chain(std::iter::once(wrapping))
        .collect()
}
