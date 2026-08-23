//! Desktop notifications, for when the window is unfocused.

/// Quote and escape a string so AppleScript reads it as one literal.
///
/// Required: an unescaped `"` ends the literal and the rest runs as AppleScript.
#[cfg(target_os = "macos")]
fn applescript_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Send one desktop notification. `urgency` is accepted and ignored — macOS has no
/// equivalent, and osascript attributes the notification to the script host, not us.
#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn notify_desktop(
    title: String,
    body: String,
    urgency: Option<String>,
) -> Result<(), String> {
    let _ = urgency;

    let script = format!(
        "display notification {} with title {}",
        applescript_string(&body),
        applescript_string(&title)
    );

    let out = tokio::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .await
        .map_err(|e| format!("osascript: {e}"))?;

    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "osascript exited with {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// Send one desktop notification. `urgency` is `low`, `normal` or `critical`.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn notify_desktop(
    title: String,
    body: String,
    urgency: Option<String>,
) -> Result<(), String> {
    // Anything unexpected becomes `normal`.
    let urgency = match urgency.as_deref() {
        Some("low") => "low",
        Some("critical") => "critical",
        _ => "normal",
    };

    let out = tokio::process::Command::new("notify-send")
        .args(["-a", "Groove", "-u", urgency, &title, &body])
        .output()
        .await
        .map_err(|e| format!("notify-send: {e} (install libnotify-bin)"))?;

    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "notify-send exited with {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn escapes_quotes_and_backslashes() {
        assert_eq!(applescript_string(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(applescript_string(r"a\b"), r#""a\\b""#);
        assert_eq!(applescript_string("plain"), r#""plain""#);
    }
}
