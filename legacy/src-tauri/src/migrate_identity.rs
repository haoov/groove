//! One-time copy of the config and data directories from the legacy bundle
//! identifiers to the current one. Nothing is deleted.

use std::path::{Path, PathBuf};

/// Legacy identifiers in priority order; the copy never overwrites, so the first source holding a file wins.
const LEGACY_IDS: [&str; 2] = ["com.rsabbah.platform-workbench", "com.rsabbah.groove"];
const NEW_ID: &str = "com.haoov.groove";
/// Marks a finished copy. Do not replace it with an emptiness check: the webview
/// creates directories in the data dir as the window opens.
const MARKER: &str = ".groove-migrated";

/// Copy `from` into `to` for any entry `to` does not already have.
/// SQLite `-shm` files are skipped; `-wal` files are copied.
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

/// Bring a renamed install's state forward, once per directory. Must run before the window is created.
pub fn from_legacy_identity(config_dir: &Path, data_dir: &Path) {
    for new_dir in [config_dir, data_dir] {
        if new_dir.join(MARKER).exists() {
            continue;
        }
        let Some(parent) = new_dir.parent() else {
            continue;
        };

        let mut failed = false;
        let mut saw_legacy = false;
        for legacy_id in LEGACY_IDS {
            let legacy = parent.join(legacy_id);
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
        // Marker only after a clean copy from an existing legacy install.
        if saw_legacy && !failed {
            let _ = std::fs::write(new_dir.join(MARKER), "");
        }
    }
}

/// The config and data directories for `NEW_ID`, as Tauri resolves them on Linux.
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

    fn setup(name: &str) -> (Tmp, std::path::PathBuf, std::path::PathBuf) {
        let root =
            std::env::temp_dir().join(format!("groove-migrate-{name}-{}", std::process::id()));
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
        assert!(
            new.join("localstorage/db.sqlite").is_file(),
            "nested dirs come too"
        );
        assert!(
            legacy.join("workbench.config.json").is_file(),
            "the old copy stays"
        );
    }

    #[test]
    fn never_overwrites_state_the_app_already_wrote() {
        let (_t, legacy, new) = setup("noclobber");
        std::fs::write(legacy.join("workbench.config.json"), "OLD").unwrap();
        std::fs::write(new.join("workbench.config.json"), "NEW").unwrap();

        from_legacy_identity(&new, &new);
        assert_eq!(
            std::fs::read_to_string(new.join("workbench.config.json")).unwrap(),
            "NEW"
        );
    }

    #[test]
    fn migrates_even_though_the_webview_already_made_directories() {
        let (_t, legacy, new) = setup("webview");
        std::fs::write(legacy.join("app.db"), "REAL DATA").unwrap();
        for d in ["localstorage", "WebKitCache", "CacheStorage"] {
            std::fs::create_dir_all(new.join(d)).unwrap();
        }
        std::fs::write(new.join("hsts-storage.sqlite"), "webkit").unwrap();

        from_legacy_identity(&new, &new);
        assert_eq!(
            std::fs::read_to_string(new.join("app.db")).unwrap(),
            "REAL DATA"
        );
    }

    #[test]
    fn runs_exactly_once() {
        let (_t, legacy, new) = setup("once");
        std::fs::write(legacy.join("app.db"), "OLD").unwrap();
        from_legacy_identity(&new, &new);
        assert!(new.join(MARKER).exists(), "the marker records that it ran");

        std::fs::remove_file(new.join("app.db")).unwrap();
        from_legacy_identity(&new, &new);
        assert!(
            !new.join("app.db").exists(),
            "a deliberate delete is not undone"
        );
    }

    #[test]
    fn does_nothing_on_a_fresh_machine() {
        let (_t, legacy, new) = setup("fresh");
        std::fs::remove_dir_all(&legacy).unwrap();
        from_legacy_identity(&new, &new);
        assert!(std::fs::read_dir(&new).unwrap().next().is_none());
    }

    #[test]
    fn the_first_legacy_id_wins_per_file() {
        let (_t, original, new) = setup("priority");
        let interim = new.parent().unwrap().join(LEGACY_IDS[1]);
        std::fs::create_dir_all(&interim).unwrap();
        std::fs::write(original.join("app.db"), "REAL").unwrap();
        std::fs::write(interim.join("app.db"), "INTERIM").unwrap();
        std::fs::write(interim.join("extra.json"), "keep me").unwrap();

        from_legacy_identity(&new, &new);
        assert_eq!(std::fs::read_to_string(new.join("app.db")).unwrap(), "REAL");
        assert_eq!(
            std::fs::read_to_string(new.join("extra.json")).unwrap(),
            "keep me"
        );
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
        assert!(
            new.join("app.db-wal").is_file(),
            "the WAL may hold recent commits"
        );
        assert!(!new.join("app.db-shm").exists(), "SQLite rebuilds this one");
    }
}
