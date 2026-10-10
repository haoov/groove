//! What a menu offers, and where it is drawn.

use groove_controllers::delivery::Say;
use groove_controllers::{Command, delivery, workspace};
use groove_types::ReviewVerdict;
use groove_ui_kit::base::style::Role;

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::{Asked, Naming, Noting};
use crate::{Corner, Losing, Menu, Of};

mod act;
pub use act::Act;
use groove_ui_kit::widgets::{menu, menu_size};

/// The actions of one file.
pub const FILE: [Act; 1] = [Act::DiscardChanges];

/// The actions of one open file's tab.
pub const TAB: [Act; 3] = [Act::Close, Act::CloseOthers, Act::CloseAll];

/// What the lines under a click offer.
pub const LINE: [Act; 1] = [Act::Note];

/// The actions of the worktree, from the commit box.
pub const WORKTREE: [Act; 3] = [Act::Push, Act::Pull, Act::DiscardEverything];

/// The same, for a worktree whose branch already has a merge request.
pub const WORKTREE_MR: [Act; 6] = [
    Act::Push,
    Act::Pull,
    Act::UpdateMr,
    Act::Comment,
    Act::CloseMr,
    Act::DiscardEverything,
];

/// What a session reviewing someone else's merge request offers.
pub const WORKTREE_REVIEW: [Act; 6] = [
    Act::Pull,
    Act::Comment,
    Act::Approve,
    Act::RequestChanges,
    Act::CloseMr,
    Act::DiscardEverything,
];

/// The actions of one path of the explorer.
pub const PATH: [Act; 5] = [
    Act::NewFile,
    Act::NewDirectory,
    Act::Rename,
    Act::Copy,
    Act::Delete,
];

/// The actions of the session, from the header.
pub const SESSION: [Act; 1] = [Act::DeleteLocally];

/// The actions a menu of fixed rows offers; none for a menu whose rows come with it.
fn acts(of: &Of) -> &'static [Act] {
    match of {
        Of::File(_) => &FILE,
        Of::Tab { .. } => &TAB,
        Of::Line { .. } => &LINE,
        Of::Path { .. } => &PATH,
        Of::Worktree { review: true, .. } => &WORKTREE_REVIEW,
        Of::Worktree { mr: true, .. } => &WORKTREE_MR,
        Of::Worktree { mr: false, .. } => &WORKTREE,
        Of::Session(_) => &SESSION,
        Of::Skills { .. } | Of::Mapping(_) | Of::Hues(_) | Of::LogRanges => &[],
    }
}

pub fn rows(of: &Of) -> Vec<&str> {
    match of {
        Of::Skills { offered, .. } => offered.iter().map(|one| one.label.as_str()).collect(),
        Of::Mapping(choices) if choices.options.is_empty() => vec![choices.empty],
        Of::Mapping(choices) => choices.options.iter().map(String::as_str).collect(),
        Of::Hues(_) => groove_types::Hue::ALL
            .iter()
            .map(|one| one.name())
            .collect(),
        Of::LogRanges => crate::views::session::resources::RANGES.to_vec(),
        _ => acts(of).iter().map(|one| one.label()).collect(),
    }
}

pub fn draw(ctx: &mut Ctx, open: &Menu) {
    let within = ctx.window;
    let rows = rows(&open.of);
    let (wide, tall) = menu_size(ctx, &rows);
    let at = match open.corner {
        Corner::TopLeft => open.at,
        Corner::BottomLeft => (open.at.0, open.at.1 - tall),
        Corner::BottomRight => (open.at.0 - wide, open.at.1 - tall),
    };
    ctx.layer();
    let edge = ctx.styles.border();
    let roles: Vec<Role> = match &open.of {
        Of::Hues(_) => groove_types::Hue::ALL
            .iter()
            .map(|one| Role::Hue(*one))
            .collect(),
        _ => Vec::new(),
    };
    menu(ctx, at, within, (&rows, &roles), edge, Target::MenuRow);
}

/// A menu whose rows come with it: a skill sent, or a mapping slot given a value.
fn offered(of: &Of, at: usize) -> Option<Picked> {
    match of {
        Of::Skills { session, offered } => Some(sent(session, offered.get(at))),
        Of::Hues(context) => {
            let hue = groove_types::Hue::ALL.get(at)?;
            let set = groove_controllers::config::Command::SetCluster {
                context: context.clone(),
                change: groove_types::ClusterChange::Hue(*hue),
            };
            Some(Picked {
                commands: vec![Command::Config(set)],
                ..Picked::default()
            })
        }
        Of::Mapping(choices) => {
            let map = |change| groove_controllers::config::Command::Map {
                source: choices.source,
                change,
            };
            let commands = choices.change(at).map(map).map(Command::Config);
            Some(Picked {
                commands: commands.into_iter().collect(),
                ..Picked::default()
            })
        }
        Of::Tab { path, owes, others } => Some(closed(path, *owes, others, at)),
        _ => None,
    }
}

/// The tabs one row of a tab's menu closes; the one clicked asks first when it owes the disk.
fn closed(path: &str, owes: bool, others: &[String], at: usize) -> Picked {
    let this = (!owes).then_some(path);
    let paths: Vec<&str> = match TAB.get(at) {
        Some(Act::Close) if owes => return Picked::asks(Losing::Tab(path.to_string())),
        Some(Act::Close) => vec![path],
        Some(Act::CloseOthers) => others.iter().map(String::as_str).collect(),
        Some(Act::CloseAll) => others.iter().map(String::as_str).chain(this).collect(),
        _ => Vec::new(),
    };
    let close = |path: &str| workspace::Command::CloseFile {
        path: path.to_string(),
    };
    Picked {
        commands: paths
            .into_iter()
            .map(close)
            .map(Command::Workspace)
            .collect(),
        ..Picked::default()
    }
}

/// One skill typed into the agent's own prompt.
fn sent(session: &groove_types::SessionId, offer: Option<&crate::Offer>) -> Picked {
    let Some(one) = offer else {
        return Picked::default();
    };
    let send = groove_controllers::agent::Command::SendSkill {
        session: session.clone(),
        id: one.id.clone(),
        args: one.args.clone(),
    };
    Picked {
        commands: vec![Command::Agent(send)],
        ..Picked::default()
    }
}

/// The directory a new path goes in: the one clicked, or the one its file stands in.
fn named(asked: Asked, path: &str, dir: bool, from: &str) -> Naming {
    let at = match dir {
        true => path.to_string(),
        false => path
            .rsplit_once('/')
            .map(|(up, _)| up)
            .unwrap_or("")
            .to_string(),
    };
    Naming::new(asked, &at, from)
}

/// What picking a menu row leaves: commands to send, and what the surface now asks.
#[derive(Debug, Default, PartialEq)]
pub struct Picked {
    pub commands: Vec<Command>,
    pub asking: Option<Losing>,
    pub naming: Option<Naming>,
    pub noting: Option<Noting>,
}

impl Picked {
    fn sends(command: workspace::Command) -> Self {
        Self {
            commands: vec![Command::Workspace(command)],
            ..Self::default()
        }
    }

    fn delivers(command: delivery::Command) -> Self {
        Self {
            commands: vec![Command::Delivery(command)],
            ..Self::default()
        }
    }

    fn asks(losing: Losing) -> Self {
        Self {
            asking: Some(losing),
            ..Self::default()
        }
    }

    fn names(naming: Naming) -> Self {
        Self {
            naming: Some(naming),
            ..Self::default()
        }
    }

    fn notes(anchor: groove_types::Anchor) -> Self {
        Self {
            noting: Some(Noting::new(anchor)),
            ..Self::default()
        }
    }
}

/// What picking row `at` of this menu does: a command, a question, or words to type.
pub fn picked(of: &Of, at: usize) -> Picked {
    if let Some(picked) = offered(of, at) {
        return picked;
    }
    match (of, acts(of).get(at)) {
        (Of::File(path), Some(Act::DiscardChanges)) => Picked::asks(Losing::File(path.clone())),
        (Of::Line { path, lines }, Some(Act::Note)) => Picked::notes(groove_types::Anchor {
            path: path.clone(),
            start_line: lines.0,
            end_line: lines.1,
        }),
        (Of::Worktree { .. }, Some(act)) => worktree(*act),
        (Of::Path { path, dir }, Some(Act::NewFile)) => {
            Picked::names(named(Asked::File, path, *dir, ""))
        }
        (Of::Path { path, dir }, Some(Act::NewDirectory)) => {
            Picked::names(named(Asked::Folder, path, *dir, ""))
        }
        (Of::Path { path, .. }, Some(Act::Rename)) => Picked::names(Naming::new(
            Asked::Rename,
            path,
            crate::views::name_of(path),
        )),
        (Of::Path { path, .. }, Some(Act::Copy)) => {
            Picked::names(Naming::new(Asked::Copy, path, crate::views::name_of(path)))
        }
        (Of::Path { path, .. }, Some(Act::Delete)) => Picked::asks(Losing::Path(path.clone())),
        (Of::Session(session), Some(Act::DeleteLocally)) => {
            let away = groove_controllers::session::Command::DeleteLocal {
                session: session.clone(),
            };
            Picked {
                commands: vec![Command::Session(away)],
                ..Picked::default()
            }
        }
        _ => Picked::default(),
    }
}

/// What picking `act` on the worktree does.
fn worktree(act: Act) -> Picked {
    match act {
        Act::DiscardEverything => Picked::asks(Losing::Everything),
        Act::Push => Picked::sends(workspace::Command::Push),
        Act::Pull => Picked::sends(workspace::Command::Pull),
        Act::UpdateMr => Picked::delivers(delivery::Command::UpdateMr),
        Act::CloseMr => Picked::delivers(delivery::Command::CloseMr),
        Act::Comment => {
            Picked::delivers(delivery::Command::Say(Say::Review(ReviewVerdict::Comment)))
        }
        Act::Approve => {
            Picked::delivers(delivery::Command::Say(Say::Review(ReviewVerdict::Approve)))
        }
        Act::RequestChanges => Picked::delivers(delivery::Command::Say(Say::Review(
            ReviewVerdict::RequestChanges,
        ))),
        _ => Picked::default(),
    }
}
