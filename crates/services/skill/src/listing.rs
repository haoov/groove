//! Where the plugins live, and one listing of the skills and routines they and the config hold.

use std::path::{Path, PathBuf};

use groove_routines::Listed;
use groove_skills::{Dirs, Plugin};
use groove_types::{Error, SharedConfig, Skill};

use crate::shared::{Shared, follow};

/// The app's plugin directories, the skills switched off, the shared plugins and those enabled.
pub fn dirs(
    (data, config): (&Path, &Path),
    off: Vec<String>,
    plugins: Option<Vec<Plugin>>,
    shared: Option<&SharedConfig>,
) -> Dirs {
    let enabled = shared.map(|one| one.enabled.clone()).unwrap_or_default();
    let plain = Dirs::new(data, config).switched(off);
    plain.sharing(plugins.unwrap_or_default(), enabled)
}

/// What one listing found: the shared copy followed, the plugins written, the skills and routines.
pub struct Listing {
    pub followed: Option<Result<Shared, Error>>,
    pub made: Result<(), Error>,
    pub skills: Vec<Skill>,
    pub routines: Vec<Listed>,
}

/// The shared copy followed, every plugin written, then every skill and routine read.
pub async fn listing(
    dirs: Dirs,
    (data, config): (PathBuf, PathBuf),
    shared: Option<SharedConfig>,
    held: Option<PathBuf>,
) -> Listing {
    let followed = match &shared {
        Some(one) => Some(follow(&data, one).await),
        None => None,
    };
    let dirs = match &followed {
        Some(Ok(copy)) => {
            let enabled = shared
                .as_ref()
                .map(|one| one.enabled.clone())
                .unwrap_or_default();
            dirs.sharing(copy.marketplace.plugins.clone(), enabled)
        }
        _ => dirs,
    };
    let copy = match &followed {
        Some(Ok(one)) => Some(one.path.clone()),
        Some(Err(_)) => held,
        None => None,
    };
    let routines = groove_routines::list(&groove_routines::Dirs::new(&config, copy.as_deref()));
    Listing {
        made: groove_skills::sync(&dirs),
        skills: groove_skills::list(&dirs),
        followed,
        routines,
    }
}

/// The shared repo's `knowledge/`, when its copy holds one.
pub fn knowledge(shared: Option<&Shared>) -> Option<PathBuf> {
    let dir = shared?.path.join("knowledge");
    dir.is_dir().then_some(dir)
}
