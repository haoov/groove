//! One-time move of the config and data directories after the app was renamed.
//!
//! The bundle identifier decides where Tauri puts both. Renaming it therefore points
//! the app at empty directories: the config is "missing" (so the setup screen
//! appears), the
//! SQLite database is new (so tasks, annotations and tracked time look gone) and the
//! webview's localStorage is fresh (so the keymap resets).
//!
//! Nothing is deleted. The old directories are left where they are and only copied
//! from, so a downgrade to the previous build still finds its state.

use std::path::{Path, PathBuf};

/// Where state may be found, newest-known first — the ORDER IS THE PRIORITY, since
/// the copy never overwrites: the first source holding a file wins.
///
/// `com.rsabbah.platform-workbench` is the original. `com.rsabbah.groove` existed
/// only between two builds and was never released, so it is second: its database
/// holds a few Notion-synced rows and nothing else, and must never shadow the real
/// one.
const LEGACY_IDS: [&str; 2] = ["com.rsabbah.platform-workbench", "com.rsabbah.groove"];
const NEW_ID: &str = "com.haoov.groove";
/// Written once the copy has run, so the decision never depends on directory
/// contents. It cannot: the webview creates `localstorage/`, `WebKitCache/` and
/// friends in the data directory as the window opens, which made an emptiness check
/// conclude the directory was already in use and skip the database.
const MARKER: &str = ".groove-migrated";

/// Copy `from` into `to` for any entry `to` does not already have.
///
/// SQLite's `-shm` file is skipped: it is shared memory rebuilt from the database
/// and the WAL, and a stale copy beside a live WAL can confuse recovery. The `-wal`
/// itself IS copied, since it may hold commits not yet in the main file.
fn copy_missing(from: &Path, to: &Path) -> std::io::Result<u32> {
    let mut copied = 0;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().ends_with("-shm") {
            continue;
        }
        let dest = to.join(name);
        if dest.exists() {
            continue;
        }
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&dest)?;
            copied += copy_missing(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
            copied += 1;
        }
    }
    Ok(copied)
}

/// Bring a renamed install's state forward, exactly once per directory.
///
/// Runs BEFORE the window is created, so nothing has written to the destination
/// yet — not the database, and not the webview's own storage. Both matter: the copy
/// never overwrites, so anything already present would win.
pub fn from_legacy_identity(config_dir: &Path, data_dir: &Path) {
    for new_dir in [config_dir, data_dir] {
        if new_dir.join(MARKER).exists() {
            continue;
        }
        let Some(parent) = new_dir.parent() else { continue };

        let mut failed = false;
        let mut saw_legacy = false;
        for legacy_id in LEGACY_IDS {
            let legacy = parent.join(legacy_id);
            // Same directory (identifier unchanged) or nothing to carry over.
            if legacy == new_dir || !legacy.is_dir() {
                continue;
            }
            saw_legacy = true;
            let _ = std::fs::create_dir_all(new_dir);
            match copy_missing(&legacy, new_dir) {
                Ok(n) => {
                    if n > 0 {
                        tracing::info!(
                            "carried {n} files forward from {} to {}",
                            legacy.display(),
                            new_dir.display()
                        );
                    }
                }
                Err(e) => {
                    failed = true;
                    tracing::warn!("could not migrate {}: {e}", legacy.display());
                }
            }
        }
        // Marker last, and only when there WAS a legacy install and the copy was
        // clean: a fresh machine stays pristine, and a failed copy retries.
        if saw_legacy && !failed {
            let _ = std::fs::write(new_dir.join(MARKER), "");
        }
    }
}

/// The config and data directories for an identifier, as Tauri resolves them on
/// Linux. Derived here rather than from the AppHandle because this has to run
/// before the app is built — `setup()` is already too late, the webview has opened
/// by then. Linux-only, which is the supported platform.
pub fn linux_dirs() -> Option<(PathBuf, PathBuf)> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    Some((config.join(NEW_ID), data.join(NEW_ID)))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tmp(std::path::PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A parent holding both the legacy and the new directory, as the real
    /// `~/.config` does.
    fn setup(name: &str) -> (Tmp, std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("groove-migrate-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let legacy = root.join(LEGACY_IDS[0]);
        let new = root.join(NEW_ID);
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        (Tmp(root), legacy, new)
    }

    #[test]
    fn carries_a_renamed_install_forward() {
        let (_t, legacy, new) = setup("carry");
        std::fs::write(legacy.join("workbench.config.json"), "{}").unwrap();
        std::fs::create_dir_all(legacy.join("localstorage")).unwrap();
        std::fs::write(legacy.join("localstorage/db.sqlite"), "x").unwrap();

        from_legacy_identity(&new, &new);
        assert!(new.join("workbench.config.json").is_file());
        assert!(new.join("localstorage/db.sqlite").is_file(), "nested dirs come too");
        assert!(legacy.join("workbench.config.json").is_file(), "the old copy stays");
    }

    /// The dangerous case: a config already written under the new identity must not
    /// be replaced by an older one.
    #[test]
    fn never_overwrites_state_the_app_already_wrote() {
        let (_t, legacy, new) = setup("noclobber");
        std::fs::write(legacy.join("workbench.config.json"), "OLD").unwrap();
        std::fs::write(new.join("workbench.config.json"), "NEW").unwrap();

        from_legacy_identity(&new, &new);
        assert_eq!(std::fs::read_to_string(new.join("workbench.config.json")).unwrap(), "NEW");
    }

    /// THE bug this file shipped with: the webview creates its own directories in
    /// the data dir before anything else runs, so "is it empty?" answered wrongly
    /// and the database never came across.
    #[test]
    fn migrates_even_though_the_webview_already_made_directories() {
        let (_t, legacy, new) = setup("webview");
        std::fs::write(legacy.join("app.db"), "REAL DATA").unwrap();
        for d in ["localstorage", "WebKitCache", "CacheStorage"] {
            std::fs::create_dir_all(new.join(d)).unwrap();
        }
        std::fs::write(new.join("hsts-storage.sqlite"), "webkit").unwrap();

        from_legacy_identity(&new, &new);
        assert_eq!(std::fs::read_to_string(new.join("app.db")).unwrap(), "REAL DATA");
    }

    /// Once migrated, a later start must not copy again — even if the user deleted
    /// something in the meantime.
    #[test]
    fn runs_exactly_once() {
        let (_t, legacy, new) = setup("once");
        std::fs::write(legacy.join("app.db"), "OLD").unwrap();
        from_legacy_identity(&new, &new);
        assert!(new.join(MARKER).exists(), "the marker records that it ran");

        std::fs::remove_file(new.join("app.db")).unwrap();
        from_legacy_identity(&new, &new);
        assert!(!new.join("app.db").exists(), "a deliberate delete is not undone");
    }

    #[test]
    fn does_nothing_on_a_fresh_machine() {
        let (_t, legacy, new) = setup("fresh");
        std::fs::remove_dir_all(&legacy).unwrap();
        from_legacy_identity(&new, &new);
        // No legacy directory means no marker either: nothing happened at all.
        assert!(std::fs::read_dir(&new).unwrap().next().is_none());
    }

    /// Two possible sources: the original must win over the short-lived one, whose
    /// database holds only re-syncable rows.
    #[test]
    fn the_first_legacy_id_wins_per_file() {
        let (_t, original, new) = setup("priority");
        let interim = new.parent().unwrap().join(LEGACY_IDS[1]);
        std::fs::create_dir_all(&interim).unwrap();
        std::fs::write(original.join("app.db"), "REAL").unwrap();
        std::fs::write(interim.join("app.db"), "INTERIM").unwrap();
        // Something only the interim directory has still comes across.
        std::fs::write(interim.join("extra.json"), "keep me").unwrap();

        from_legacy_identity(&new, &new);
        assert_eq!(std::fs::read_to_string(new.join("app.db")).unwrap(), "REAL");
        assert_eq!(std::fs::read_to_string(new.join("extra.json")).unwrap(), "keep me");
    }

    #[test]
    fn derives_the_linux_directories_from_the_environment() {
        let (config, data) = linux_dirs().expect("HOME is set");
        assert!(config.ends_with(NEW_ID), "{}", config.display());
        assert!(data.ends_with(NEW_ID), "{}", data.display());
        assert_ne!(config, data);
    }

    #[test]
    fn skips_the_sqlite_shared_memory_file() {
        let (_t, legacy, new) = setup("shm");
        for f in ["app.db", "app.db-wal", "app.db-shm"] {
            std::fs::write(legacy.join(f), "x").unwrap();
        }
        from_legacy_identity(&new, &new);
        assert!(new.join("app.db").is_file());
        assert!(new.join("app.db-wal").is_file(), "the WAL may hold recent commits");
        assert!(!new.join("app.db-shm").exists(), "SQLite rebuilds this one");
    }


}
