//! Turning a task source on with its config, or off.

use groove_types::{Config, Error, GithubConfig, NotionConfig, ProviderId};

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
        match source {
            Source::Notion(block) => next.notion = block,
            Source::Github(block) => next.github = block,
        }
        if next.notion.is_none() && next.github.is_none() {
            return Err(Error::invalid("that would leave no task source at all"));
        }
        *config = next;
        Ok(config)
    }
}
