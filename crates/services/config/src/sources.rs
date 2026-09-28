//! Turning a task source on with its config, or off, and mapping its names and values.

use groove_types::{Config, Error, GithubConfig, Mapping, NotionConfig, Property, ProviderId};

use crate::State;

/// One source's block: the config to hold, or `None` to turn it off.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    Notion(Option<NotionConfig>),
    Github(Option<GithubConfig>),
}

impl Source {
    pub fn id(&self) -> ProviderId {
        match self {
            Source::Notion(_) => ProviderId::Notion,
            Source::Github(_) => ProviderId::Github,
        }
    }
}

impl State {
    /// The source's block replaced; the config to write. The last source stays on.
    pub fn set_source(&mut self, source: Source) -> Result<&Config, Error> {
        let config = self
            .config
            .as_mut()
            .ok_or_else(|| Error::invalid("there is no config to change before the first run"))?;
        let mut next = config.clone();
        let source_id = source.id();
        match source {
            Source::Notion(block) => next.notion = block,
            Source::Github(block) => next.github = block,
        }
        if next.notion.is_none() && next.github.is_none() {
            return Err(Error::invalid("that would leave no task source at all"));
        }
        *config = next;
        self.schemas.retain(|(id, _)| *id != source_id);
        Ok(config)
    }

    /// One change to a source's mapping; the config to write.
    pub fn map(&mut self, source: ProviderId, change: &Mapping) -> Result<&Config, Error> {
        let config = self
            .config
            .as_mut()
            .ok_or_else(|| Error::invalid("there is no config to change before the first run"))?;
        let off = || Error::invalid(format!("{} is not on", source.label()));
        match source {
            ProviderId::Notion => config.notion.as_mut().ok_or_else(off)?.map(change),
            ProviderId::Github => config.github.as_mut().ok_or_else(off)?.map(change),
        }
        Ok(config)
    }

    /// What one source was last read to hold.
    pub fn schema(&self, source: ProviderId) -> Option<&[Property]> {
        let held = self.schemas.iter().find(|(id, _)| *id == source);
        held.map(|(_, properties)| properties.as_slice())
    }
}
