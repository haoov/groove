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
}
