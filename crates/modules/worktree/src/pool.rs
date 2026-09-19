//! The clones every worktree is cut from.

use std::path::Path;

use groove_git::{Git, RemoteUrl};
use groove_types::{PoolEntry, Repo, RepoId};

use crate::{Error, Pool, Result};

const WALK_DEPTH: u32 = 6;

impl Pool {
    /// Every clone in the pool, by directory walk, sorted by slug. No pool is an empty list.
    pub fn list(&self) -> Vec<PoolEntry> {
        let main = self.layout.main();
        let mut clones = Vec::new();
        walk(&main, &main, 1, &mut clones);
        clones.sort_by_key(|c| c.slug.to_lowercase());
        clones
    }

    /// The one clone `name` means: its whole slug, or any suffix on a `/` boundary.
    pub fn resolve<'a>(name: &str, clones: &'a [PoolEntry]) -> Result<&'a PoolEntry> {
        let name = name.trim().trim_matches('/');
        let matches: Vec<&PoolEntry> = clones
            .iter()
            .filter(|c| c.slug == name || c.slug.ends_with(&format!("/{name}")))
            .collect();
        match matches.as_slice() {
            [one] => Ok(one),
            [] => Err(Error::UnknownRepo(name.to_string())),
            many => Err(Error::AmbiguousRepo {
                name: name.to_string(),
                matches: many.iter().map(|c| c.slug.clone()).collect(),
            }),
        }
    }

    /// Records a clone as a repo. The one git call checks it has an origin.
    pub async fn register(&self, clone: &PoolEntry) -> Result<Repo> {
        let (host, group, project) = slug_parts(&clone.slug)?;
        if !Git::at(&clone.path).has_remote("origin").await? {
            return Err(Error::NoOrigin {
                path: clone.path.clone(),
            });
        }
        let repo = Repo {
            id: RepoId::new(&clone.slug),
            host,
            group_path: group,
            project,
            local_path: clone.path.to_string_lossy().into_owned(),
        };
        self.upsert_repo(&repo).await?;
        Ok(repo)
    }

    /// Clones `url` into its place in the pool and records it.
    pub async fn clone(&self, url: &str) -> Result<Repo> {
        let remote = RemoteUrl::parse(url)?;
        let dest = self
            .layout
            .repo_dir(&remote.host, &remote.group, &remote.project);
        if dest.exists() {
            return Err(Error::Exists { path: dest });
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|source| Error::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        Git::clone(url, &dest).await?;
        self.register(&PoolEntry {
            slug: remote.slug(),
            path: dest,
        })
        .await
    }
}

/// `(host, group, project)` off a pool slug.
fn slug_parts(slug: &str) -> Result<(String, String, String)> {
    let mut segments: Vec<&str> = slug.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 3 {
        return Err(Error::BadSlug(slug.to_string()));
    }
    let project = segments.pop().unwrap_or_default().to_string();
    let host = segments.remove(0).to_string();
    Ok((host, segments.join("/"), project))
}

/// Directories holding a `.git`, at most `WALK_DEPTH` deep; a clone's own tree is not entered.
fn walk(dir: &Path, root: &Path, depth: u32, acc: &mut Vec<PoolEntry>) {
    if depth > WALK_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.join(".git").exists() {
            if let Ok(slug) = path.strip_prefix(root) {
                acc.push(PoolEntry {
                    slug: slug.to_string_lossy().into_owned(),
                    path,
                });
            }
        } else {
            walk(&path, root, depth + 1, acc);
        }
    }
}
