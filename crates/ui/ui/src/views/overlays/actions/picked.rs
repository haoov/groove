//! What picking a row of a menu does: the commands it sends, or what the surface asks next.

use groove_controllers::{Command, session, workspace};
use groove_types::Section;

use super::{Act, TAB, acts};
use crate::views::session::{Asked, Naming, Noting};
use crate::{Losing, Of};

/// A menu whose rows come with it: a write asked, a skill sent, or a mapping slot given a value.
fn offered(of: &Of, at: usize) -> Option<Picked> {
    match of {
        Of::Resource {
            session,
            shells,
            writes,
            offered,
        } => Some(match at.checked_sub(shells.len()) {
            None => shelled(session, &shells[at]),
            Some(at) => match writes.get(at) {
                Some(Losing::Row(written)) if written.write.immediate() => Picked {
                    commands: vec![(**written).clone().command()],
                    ..Picked::default()
                },
                Some(write) => Picked::asks(write.clone()),
                None => sent(session, offered.get(at - writes.len())),
            },
        }),
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

/// A shell in a pod, in a tab of the drawer, opened to take the keys.
fn shelled(session: &groove_types::SessionId, one: &crate::Shelled) -> Picked {
    let shell = groove_controllers::shell::Command::Pod {
        session: session.clone(),
        pod: one.pod.clone(),
        container: one.container.clone(),
    };
    Picked {
        commands: vec![Command::Shell(shell)],
        drawer: true,
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
    /// The terminal drawer opens, its new tab taking the keys.
    pub drawer: bool,
}

impl Picked {
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

/// A session paused, resumed, moved to the top of its section, or closed.
fn placed(session: &groove_types::SessionId, section: Option<Section>, act: Act) -> Picked {
    let session = session.clone();
    let place = |section, index| session::Command::Place {
        session: session.clone(),
        section,
        index,
    };
    let command = match (act, section) {
        (Act::Close, _) => session::Command::Close { session },
        (Act::Pause, _) => place(Section::Paused, 0),
        (Act::Resume, _) => place(Section::Active, 0),
        (Act::MoveToTop, Some(section)) => place(section, 0),
        _ => return Picked::default(),
    };
    Picked {
        commands: vec![Command::Session(command)],
        ..Picked::default()
    }
}

/// What picking row `at` of this menu does: a command, a question, or words to type.
pub fn picked(of: &Of, at: usize) -> Picked {
    if let Some(picked) = offered(of, at) {
        return picked;
    }
    match (of, acts(of).get(at)) {
        (Of::Line { path, lines }, Some(Act::Note)) => Picked::notes(groove_types::Anchor {
            path: path.clone(),
            start_line: lines.0,
            end_line: lines.1,
        }),
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
        (Of::Session { session, section }, Some(act)) => placed(session, *section, *act),
        _ => Picked::default(),
    }
}
