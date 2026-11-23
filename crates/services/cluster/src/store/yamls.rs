//! Each object followed by name as read-only text, and the draft of one the user writes.

use std::collections::BTreeMap;

use groove_text::{Buffer, Document};
use groove_types::{Described, Edit, FollowKey};

#[derive(Debug, Default)]
pub struct Yamls {
    buffers: BTreeMap<FollowKey, Buffer>,
    drafts: BTreeMap<FollowKey, Draft>,
}

/// An object's YAML being written: the text it started from, and the text as written.
#[derive(Debug, Clone)]
pub struct Draft {
    pub base: String,
    pub buffer: Buffer,
}

impl Draft {
    pub fn edited(&self) -> bool {
        self.buffer.text() != self.base
    }
}

impl Yamls {
    /// The text shown: the draft while one is written, else the object as it stands.
    pub fn get(&self, key: &FollowKey) -> Option<&Buffer> {
        let draft = self.drafts.get(key).map(|one| &one.buffer);
        draft.or_else(|| self.buffers.get(key))
    }

    pub fn draft(&self, key: &FollowKey) -> Option<&Draft> {
        self.drafts.get(key)
    }

    /// Whether the object moved on since its draft started.
    pub fn moved(&self, key: &FollowKey) -> bool {
        let live = self.buffers.get(key).map(Buffer::text);
        self.drafts
            .get(key)
            .is_some_and(|one| live.is_some_and(|live| live != one.base))
    }

    /// A draft started from the object as it stands, in place of any before it.
    pub fn write(&mut self, key: &FollowKey) -> bool {
        let Some(live) = self.buffers.get(key) else {
            return false;
        };
        let draft = Draft {
            base: live.text(),
            buffer: live.clone(),
        };
        self.drafts.insert(key.clone(), draft);
        true
    }

    /// Every draft of `target` thrown away, once its edit landed.
    pub fn landed(&mut self, target: &groove_types::Target) {
        self.drafts.retain(|key, _| !target.is(key));
    }

    /// Back to the object as it stands, the draft thrown away.
    pub fn read_only(&mut self, key: &FollowKey) {
        self.drafts.remove(key);
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

    /// Any edit of a draft; else a move of the caret or of what it holds, a change refused.
    pub fn edit(&mut self, key: &FollowKey, edit: &Edit) -> bool {
        if let Some(draft) = self.drafts.get_mut(key) {
            draft.buffer.edit(edit);
            return true;
        }
        match self.buffers.get_mut(key).filter(|_| edit.moves()) {
            Some(buffer) => {
                buffer.edit(edit);
                true
            }
            None => false,
        }
    }

    pub(super) fn drop(&mut self, key: &FollowKey) {
        self.buffers.remove(key);
        self.drafts.remove(key);
    }
}
