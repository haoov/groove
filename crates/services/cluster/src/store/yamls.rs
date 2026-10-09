//! Each object followed by name as a read-only text, with a caret and a selection of its own.

use std::collections::BTreeMap;

use groove_text::{Buffer, Document};
use groove_types::{Described, Edit, FollowKey};

#[derive(Debug, Default)]
pub struct Yamls {
    buffers: BTreeMap<FollowKey, Buffer>,
}

impl Yamls {
    pub fn get(&self, key: &FollowKey) -> Option<&Buffer> {
        self.buffers.get(key)
    }

    /// The text read again from the object as it stands, what was held kept where it still fits.
    pub(super) fn refresh(&mut self, key: &FollowKey, object: Option<&Described>) {
        let Some(object) = object.filter(|one| !one.yaml.is_empty()) else {
            self.buffers.remove(key);
            return;
        };
        if self
            .buffers
            .get(key)
            .is_some_and(|held| held.text() == *object.yaml)
        {
            return;
        }
        let held = self.buffers.get(key).map(|one| one.selections()[0]);
        let mut buffer = Buffer::new(Document::new(
            &format!("{}.yaml", object.name),
            &object.yaml,
        ));
        if let Some(held) = held {
            buffer.holding(held);
        }
        self.buffers.insert(key.clone(), buffer);
    }

    /// A move of the caret or of what it holds; anything that would change the text is refused.
    pub fn edit(&mut self, key: &FollowKey, edit: &Edit) -> bool {
        let moves = matches!(
            edit,
            Edit::Move(_) | Edit::Extend(_) | Edit::SelectAll | Edit::SelectWord | Edit::SelectLine
        );
        match self.buffers.get_mut(key).filter(|_| moves) {
            Some(buffer) => {
                buffer.edit(edit);
                true
            }
            None => false,
        }
    }

    pub(super) fn drop(&mut self, key: &FollowKey) {
        self.buffers.remove(key);
    }
}
