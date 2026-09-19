//! A remote's URL, as the host, the group and the project in it.

use crate::{Error, Result};

/// A remote as the pool names it: `<host>/<group…>/<project>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteUrl {
    pub host: String,
    pub group: String,
    pub project: String,
}

impl RemoteUrl {
    /// `git@host:group/project.git`, `ssh://git@host/group/project` and `https://host/group/project`,
    /// userinfo stripped, `.git` dropped.
    pub fn parse(url: &str) -> Result<Self> {
        let url = url.trim().trim_end_matches('/').trim_end_matches(".git");
        let parsed = match url.split_once("://") {
            Some((_, rest)) => Self::from_host_path(strip_userinfo(rest), '/'),
            None => url
                .split_once('@')
                .and_then(|(_, rest)| Self::from_host_path(rest, ':')),
        };
        parsed.ok_or_else(|| Error::BadUrl(url.to_string()))
    }

    fn from_host_path(rest: &str, sep: char) -> Option<Self> {
        let (host, path) = rest.split_once(sep)?;
        let host = host.rsplit_once(':').map_or(host, |(h, port)| {
            if port.chars().all(|c| c.is_ascii_digit()) {
                h
            } else {
                host
            }
        });
        let (group, project) = path.trim_matches('/').rsplit_once('/')?;
        (!host.is_empty() && !group.is_empty() && !project.is_empty()).then(|| Self {
            host: host.to_string(),
            group: group.to_string(),
            project: project.to_string(),
        })
    }

    pub fn slug(&self) -> String {
        format!("{}/{}/{}", self.host, self.group, self.project)
    }
}

fn strip_userinfo(rest: &str) -> &str {
    rest.rsplit_once('@').map_or(rest, |(_, r)| r)
}
