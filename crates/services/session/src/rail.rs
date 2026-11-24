//! The rail's order: two sections the user sorts by hand, each child under its parent.

use std::cmp::Reverse;

use groove_types::{Section, SessionId};

use crate::{Open, State};

/// Where one session stands now: its section and its rank there, as the store keeps them.
pub type Place = (SessionId, Section, Option<i64>);

impl State {
    /// Whether `open` stands under a parent the rail holds.
    pub fn under_parent(&self, open: &Open) -> bool {
        let parent = open.state.parent.as_ref();
        parent.is_some_and(|one| self.get(one).is_some())
    }

    /// The sessions the rail lists at the top of `section`, in the user's order.
    /// The unranked stand first, the last opened on top.
    pub fn tops(&self, section: Section) -> Vec<&Open> {
        let mut tops: Vec<(usize, &Open)> = self
            .open
            .iter()
            .enumerate()
            .filter(|(_, one)| !self.under_parent(one) && one.state.section == section)
            .collect();
        tops.sort_by_key(|(at, one)| {
            let rank = one.state.rank;
            (
                rank.is_some(),
                rank,
                Reverse(one.state.opened_at),
                Reverse(*at),
            )
        });
        tops.into_iter().map(|(_, one)| one).collect()
    }

    /// The children of `parent` the rail holds, in the order opened.
    pub fn children_of(&self, parent: &SessionId) -> Vec<&Open> {
        let mine = |one: &&Open| one.state.parent.as_ref() == Some(parent);
        self.open.iter().filter(mine).collect()
    }

    /// `id` put at `index` of `section`, the section ranked again from zero.
    /// Returns every place that changed; nothing for a child, which follows its parent.
    pub fn place(&mut self, id: &SessionId, section: Section, index: usize) -> Vec<Place> {
        let Some(open) = self.get(id) else {
            return Vec::new();
        };
        if self.under_parent(open) {
            return Vec::new();
        }
        let mut order: Vec<SessionId> = self
            .tops(section)
            .into_iter()
            .map(|one| one.session.id.clone())
            .filter(|one| one != id)
            .collect();
        order.insert(index.min(order.len()), id.clone());
        let mut places = Vec::new();
        for (rank, one) in order.into_iter().enumerate() {
            let rank = Some(rank as i64);
            let Some(open) = self.get_mut(&one) else {
                continue;
            };
            if open.state.section != section || open.state.rank != rank {
                open.state.section = section;
                open.state.rank = rank;
                places.push((one, section, rank));
            }
        }
        places
    }
}
