//! Desktop notifications, for when the window is not the thing you are looking at.
//!
//! `notify-send` rather than a Tauri plugin, for the same reasons as the clipboard:
//! it needs no dependency, and it is verifiable — this desktop runs `mako` on
//! `org.freedesktop.Notifications`, so a send either appears or reports why.
//!
//! Deliberately narrow. The in-app toast already covers everything while the window
//! has focus; the only thing worth interrupting the desktop for is an agent that
//! cannot continue without the user, or a failure they have not seen.

/// Send one desktop notification. `urgency` is `low`, `normal` or `critical`.
#[tauri::command]
pub async fn notify_desktop(
    title: String,
    body: String,
    urgency: Option<String>,
) -> Result<(), String> {
    // Anything unexpected becomes `normal` rather than an error: a notification is
    // not worth failing a caller over.
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
