//! The skill capability: the skills both plugins offer, the team's shared repo and its copy.

pub mod listing;
mod offers;
pub mod shared;

#[cfg(test)]
mod tests;

use groove_types::{Error, SessionKind, Skill, Timestamp};

pub use groove_skills as skills;
pub use offers::SkillOffer;

/// The `skill` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    /// Every skill both plugins offer, as the last read found them.
    pub skills: Vec<Skill>,
    /// The copy of the team's shared repo, once it reads as a marketplace.
    pub shared: Option<shared::Shared>,
    /// A shared repo is being copied, and why the last one was refused.
    pub joining: bool,
    pub unshared: Option<String>,
    /// When a skill was last switched on or off.
    pub switched: Timestamp,
    /// The skills have been read once.
    pub listed: bool,
    /// A listing is out, and another was asked for meanwhile.
    pub listing: bool,
    pub again: bool,
    /// Moves on each time the shared repo is joined or left.
    pub generation: u64,
    /// The copy waits to be taken away until the listing out lands.
    pub unsharing: bool,
}

impl State {
    /// The skills a session of this kind offers, in the order they are listed.
    pub fn skills_for(&self, kind: &SessionKind) -> Vec<&Skill> {
        self.skills
            .iter()
            .filter(|one| one.enabled && one.offered_to(kind))
            .collect()
    }

    /// The shared repo joined with its copy, or left with none.
    pub fn reshared(&mut self, copy: Option<shared::Shared>) {
        self.shared = copy;
        self.generation += 1;
    }

    /// What a follow found kept: the copy, or the last one; a failure to say, once.
    pub fn followed(
        &mut self,
        read: Option<groove_types::Result<shared::Shared>>,
    ) -> Option<Error> {
        match read {
            None => self.shared = None,
            Some(Ok(copy)) => {
                self.shared = Some(copy);
                self.unshared = None;
            }
            Some(Err(e)) => {
                let again = self.unshared.as_deref() == Some(e.message.as_str());
                self.unshared = Some(e.message.clone());
                return (!again).then_some(e);
            }
        }
        None
    }

    /// Whether a skill changed after an agent `started`.
    pub fn newer_than(&self, started: Timestamp) -> bool {
        let changed = |one: &Skill| one.enabled && one.changed_at > started;
        self.switched > started || self.skills.iter().any(changed)
    }
}
