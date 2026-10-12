//! What the config turns on beside the app itself: the shared repo, the routines, the skills.

use groove_types::{Config, Error};

use crate::State;

impl State {
    /// The shared repo named, or none; the config to write, or `None` before first run.
    pub fn set_shared(&mut self, shared: Option<groove_types::SharedConfig>) -> Option<&Config> {
        let config = self.config.as_mut()?;
        config.shared = shared;
        Some(config)
    }

    /// The shared repo asked for, on `main` unless named, keeping its skills while the URL stays.
    /// Refused before the first run, which has no config to name it in.
    pub fn wanted_shared(
        &self,
        url: &str,
        branch: &str,
    ) -> Result<groove_types::SharedConfig, Error> {
        if self.config.is_none() {
            return Err(Error::invalid(
                "there is no config to change before the first run",
            ));
        }
        let url = url.trim().to_string();
        if url.is_empty() {
            return Err(Error::invalid("a shared repo needs its URL"));
        }
        let branch = match branch.trim() {
            "" => "main".to_string(),
            named => named.to_string(),
        };
        let same = self.shared().filter(|held| held.url == url);
        let enabled = same.map(|held| held.enabled.clone()).unwrap_or_default();
        Ok(groove_types::SharedConfig {
            url,
            branch,
            enabled,
        })
    }

    pub fn shared(&self) -> Option<&groove_types::SharedConfig> {
        self.config.as_ref()?.shared.as_ref()
    }

    pub fn routines(&self) -> groove_types::RoutinesConfig {
        self.config
            .as_ref()
            .map(|one| one.routines.clone())
            .unwrap_or_default()
    }

    /// One routine switched on or off; switched off, the triggers it had turned off are forgotten.
    pub fn switch_routine(&mut self, id: &str, on: bool) -> Option<&Config> {
        let held = &mut self.config.as_mut()?.routines;
        held.on.retain(|one| one != id);
        match on {
            true => held.on.push(id.to_string()),
            false => drop(held.quiet.remove(id)),
        }
        self.config.as_ref()
    }

    /// One trigger of a routine turned on or off.
    pub fn switch_trigger(
        &mut self,
        id: &str,
        trigger: groove_types::Trigger,
        on: bool,
    ) -> Option<&Config> {
        let held = &mut self.config.as_mut()?.routines;
        let quiet = held.quiet.entry(id.to_string()).or_default();
        quiet.retain(|one| *one != trigger);
        if !on {
            quiet.push(trigger);
        }
        if quiet.is_empty() {
            held.quiet.remove(id);
        }
        self.config.as_ref()
    }

    pub fn skills_off(&self) -> &[String] {
        self.config
            .as_ref()
            .map_or(&[], |one| one.skills_off.as_slice())
    }

    /// One skill switched on or off: a user's skill in `skills_off`, a shared one in `enabled`.
    pub fn switch_skill(&mut self, id: &str, on: bool) -> Result<&Config, Error> {
        if id.starts_with("groove:") {
            return Err(Error::invalid("a core skill is always on"));
        }
        let unknown = || {
            let why = format!("`{id}` is no skill Groove can switch: no shared repo holds it");
            Error::invalid(why)
        };
        let first = || Error::invalid("there is no config to change before the first run");
        let config = self.config.as_mut().ok_or_else(first)?;
        let (held, wanted) = match id.starts_with("user:") {
            true => (&mut config.skills_off, !on),
            false => (&mut config.shared.as_mut().ok_or_else(unknown)?.enabled, on),
        };
        held.retain(|one| one != id);
        if wanted {
            held.push(id.to_string());
        }
        Ok(config)
    }
}
