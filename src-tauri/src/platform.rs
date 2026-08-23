/// `"macos"`, `"linux"` or `"windows"`, fixed at compile time for the build target.
#[tauri::command]
pub fn platform() -> String {
    std::env::consts::OS.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_a_name_the_frontend_knows() {
        assert!(
            matches!(platform().as_str(), "macos" | "linux" | "windows"),
            "unexpected platform name: {}",
            platform()
        );
    }

    fn window_block(file: &str) -> serde_json::Map<String, serde_json::Value> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
        let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{file}: {e}"));
        let cfg: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
        cfg["app"]["windows"][0]
            .as_object()
            .unwrap_or_else(|| panic!("{file} has no app.windows[0]"))
            .clone()
    }

    /// Tauri merges platform config with JSON Merge Patch, which replaces arrays
    /// wholesale — so a base key missing from the macOS window is dropped silently.
    #[test]
    fn the_macos_window_repeats_every_key_of_the_base_window() {
        let base = window_block("tauri.conf.json");
        let macos = window_block("tauri.macos.conf.json");

        let missing: Vec<&String> = base.keys().filter(|k| !macos.contains_key(*k)).collect();
        assert!(
            missing.is_empty(),
            "tauri.macos.conf.json drops {missing:?}; every base key must be repeated there"
        );
    }

    #[test]
    fn the_macos_window_is_decorated() {
        let macos = window_block("tauri.macos.conf.json");
        assert_eq!(macos["decorations"], serde_json::json!(true));
        assert_eq!(macos["titleBarStyle"], serde_json::json!("Overlay"));
    }
}
