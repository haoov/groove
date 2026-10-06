//! What a click on Settings › Clusters does: add, remove or switch a context.

use groove_controllers::{Command, config};

use crate::hit::Target;

pub(super) fn acted(target: &Target) -> Option<Vec<Command>> {
    let asked = match target {
        Target::ClusterAdd(context) => config::Command::AddCluster {
            context: context.clone(),
        },
        Target::ClusterRemove(context) => config::Command::RemoveCluster {
            context: context.clone(),
        },
        Target::ClusterSet(context, change) => config::Command::SetCluster {
            context: context.clone(),
            change: change.clone(),
        },
        _ => return None,
    };
    Some(vec![Command::Config(asked)])
}
