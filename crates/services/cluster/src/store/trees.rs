//! Each Application's resource health from Argo CD, followed while something reads it.

use groove_objects::{Stop, Treed};

use super::follows::Named;

#[derive(Debug, Default)]
pub struct Trees {
    streams: super::leases::Leases<Named, Option<Treed>>,
}

impl Trees {
    /// The health last read, or what stopped the read; none before the first answer.
    pub fn get(&self, app: &Named) -> Option<&Treed> {
        self.streams.get(app)?.as_ref()
    }

    pub fn read_by<'a>(&'a self, reader: &'a str) -> impl Iterator<Item = &'a Named> {
        self.streams.read_by(reader)
    }

    /// `reader` reads `app`; the stop of a stream to start, when none ran for it.
    pub fn lease(&mut self, app: &Named, reader: &str) -> Option<&Stop> {
        self.streams.lease(app, reader, || None)
    }

    pub fn release(&mut self, reader: &str) -> Vec<(Named, u64)> {
        self.streams.release(reader)
    }

    pub fn drop_unread(&mut self, app: &Named, released: u64) {
        self.streams.drop_unread(app, released);
    }

    /// A tree, or a refusal, of a stream still held; one dropped meanwhile is ignored.
    pub fn apply(&mut self, app: &Named, read: Treed) {
        if let Some(held) = self.streams.get_mut(app) {
            *held = Some(read);
        }
    }
}
