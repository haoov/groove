//! What the resource tabs read: whole objects, followed while read, and the reads made once.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use groove_objects::{Change, Followed, Stop};
use groove_types::{Described, FollowKey, Timestamp, Usage};

/// One watcher of whole objects, and how it stands.
#[derive(Debug, Default)]
pub struct Follow {
    pub objects: Vec<Arc<Described>>,
    pub synced: bool,
    pub failed: Option<String>,
    readers: BTreeSet<String>,
    stop: Stop,
}

/// Context, namespace and name: one pod, or one Helm release.
pub type Named = (String, String, String);

#[derive(Debug, Default)]
pub struct Follows {
    followed: BTreeMap<FollowKey, Follow>,
    pub yamls: super::yamls::Yamls,
    /// Each pod's last usage and when it was read; none where no metrics-server answers.
    usage: BTreeMap<Named, (Option<Usage>, Timestamp)>,
    helm: BTreeMap<Named, Option<u32>>,
    asking: BTreeSet<Named>,
}

impl Follows {
    pub fn get(&self, key: &FollowKey) -> Option<&Follow> {
        self.followed.get(key)
    }

    /// Every key `reader` reads.
    pub fn read_by<'a>(&'a self, reader: &'a str) -> impl Iterator<Item = &'a FollowKey> {
        let reads = move |(key, one): (&'a FollowKey, &'a Follow)| {
            one.readers.contains(reader).then_some(key)
        };
        self.followed.iter().filter_map(reads)
    }

    /// `reader` reads `key`; the stop of a watcher to start, when none ran for it.
    pub fn lease(&mut self, key: &FollowKey, reader: &str) -> Option<&Stop> {
        let started = !self.followed.contains_key(key);
        let one = self.followed.entry(key.clone()).or_default();
        one.readers.insert(reader.to_string());
        started.then_some(&one.stop)
    }

    /// `reader` reads nothing any more; the keys it leaves without a reader.
    pub fn release(&mut self, reader: &str) -> Vec<FollowKey> {
        let mut unread = Vec::new();
        for (key, one) in &mut self.followed {
            if one.readers.remove(reader) && one.readers.is_empty() {
                unread.push(key.clone());
            }
        }
        unread
    }

    /// The watcher stopped and its objects dropped, if nothing reads it still.
    pub fn drop_unread(&mut self, key: &FollowKey) {
        if self
            .followed
            .get(key)
            .is_some_and(|one| one.readers.is_empty())
            && let Some(one) = self.followed.remove(key)
        {
            one.stop.stop();
            self.yamls.drop(key);
        }
    }

    /// A batch of a watcher still held; one dropped meanwhile is ignored.
    pub fn apply(&mut self, key: &FollowKey, batch: Followed) {
        self.landed(key, batch);
        let whole = key
            .fields
            .as_deref()
            .is_some_and(|one| one.starts_with("metadata.name="));
        if whole {
            let first = self.followed.get(key).and_then(|one| one.objects.first());
            self.yamls.refresh(key, first.map(|one| &**one));
        }
    }

    fn landed(&mut self, key: &FollowKey, batch: Followed) {
        let Some(one) = self.followed.get_mut(key) else {
            return;
        };
        match batch {
            Followed::Reset(objects) => {
                one.objects = objects.into_iter().map(Arc::new).collect();
                one.synced = true;
            }
            Followed::Changes(changes) => {
                for change in changes {
                    match change {
                        Change::Put(object) => {
                            one.objects.retain(|held| held.uid != object.uid);
                            one.objects.push(Arc::from(object));
                        }
                        Change::Gone(uid) => one.objects.retain(|held| held.uid != uid),
                    }
                }
            }
            Followed::Watching => one.failed = None,
            Followed::Failed(why) => one.failed = Some(why),
        }
    }

    pub fn usage(&self, pod: &Named) -> Option<&(Option<Usage>, Timestamp)> {
        self.usage.get(pod)
    }

    pub fn helm(&self, release: &Named) -> Option<Option<u32>> {
        self.helm.get(release).copied()
    }

    pub fn asking(&self, named: &Named) -> bool {
        self.asking.contains(named)
    }

    /// Marks a read of `named` begun. False while one already runs.
    pub fn begin(&mut self, named: &Named) -> bool {
        self.asking.insert(named.clone())
    }

    pub fn set_usage(&mut self, pod: Named, usage: Option<Usage>, at: Timestamp) {
        self.asking.remove(&pod);
        self.usage.insert(pod, (usage, at));
    }

    pub fn set_helm(&mut self, release: Named, revision: Option<u32>) {
        self.asking.remove(&release);
        self.helm.insert(release, revision);
    }

    /// A read that failed, to be asked again later.
    pub fn end(&mut self, named: &Named) {
        self.asking.remove(named);
    }
}
