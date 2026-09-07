//! System clipboard through OS tools, not the webview.

use std::time::Duration;
use tokio::io::AsyncWriteExt;

use crate::core::error::AppResult;

/// A tool that has not answered by then is killed and the next one runs.
const TOOL_TIMEOUT: Duration = Duration::from_secs(2);

/// Writers, in preference order. Linux: Wayland-native, then X11, then xsel.
#[cfg(target_os = "macos")]
const WRITERS: [(&str, &[&str]); 1] = [("pbcopy", &[])];
#[cfg(target_os = "macos")]
const READERS: [(&str, &[&str]); 1] = [("pbpaste", &[])];

#[cfg(not(target_os = "macos"))]
const WRITERS: [(&str, &[&str]); 3] = [
    ("wl-copy", &[]),
    ("xclip", &["-selection", "clipboard"]),
    ("xsel", &["--clipboard", "--input"]),
];

#[cfg(not(target_os = "macos"))]
const READERS: [(&str, &[&str]); 3] = [
    ("wl-paste", &["--no-newline"]),
    ("xclip", &["-selection", "clipboard", "-o"]),
    ("xsel", &["--clipboard", "--output"]),
];

fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// The tools that fit the session; the wl-clipboard pair needs a Wayland display.
fn session_tools(
    all: &[(&'static str, &'static [&'static str])],
    wayland: bool,
) -> Vec<(&'static str, &'static [&'static str])> {
    all.iter()
        .copied()
        .filter(|(bin, _)| wayland || !bin.starts_with("wl-"))
        .collect()
}

fn missing_tools(candidates: &[(&str, &[&str])]) -> String {
    let names: Vec<&str> = candidates.iter().map(|(bin, _)| *bin).collect();
    format!(
        "no clipboard tool found — install one of: {}",
        names.join(", ")
    )
}

/// Write `text` to the clipboard with the first tool that succeeds. One write only:
/// under mutter a second write through xclip makes a later `wl-paste` block for ever.
#[tauri::command]
pub async fn copy_to_clipboard(text: String) -> AppResult<()> {
    Ok(write_first(&session_tools(&WRITERS, is_wayland()), TOOL_TIMEOUT, &text).await?)
}

async fn write_first(
    candidates: &[(&str, &[&str])],
    limit: Duration,
    text: &str,
) -> anyhow::Result<()> {
    let mut errors: Vec<String> = vec![];

    for (bin, args) in candidates {
        let spawned = tokio::process::Command::new(bin)
            .args(*args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn();

        // Not installed is not an error.
        let Ok(mut child) = spawned else { continue };

        match tokio::time::timeout(limit, feed_and_wait(&mut child, text)).await {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(e)) => errors.push(format!("{bin}: {e}")),
            Err(_) => errors.push(format!("{bin}: timed out after {limit:?}")),
        }
    }

    Err(anyhow::anyhow!(if errors.is_empty() {
        missing_tools(candidates)
    } else {
        errors.join("; ")
    }))
}

async fn feed_and_wait(child: &mut tokio::process::Child, text: &str) -> anyhow::Result<()> {
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).await?;
        // The tool copies until stdin closes.
        drop(stdin);
    }
    match child.wait().await {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(anyhow::anyhow!("exited with {status}")),
        Err(e) => Err(e.into()),
    }
}

/// Read the system clipboard. Empty string when it holds nothing usable.
#[tauri::command]
pub async fn read_clipboard() -> AppResult<String> {
    Ok(read_first(&session_tools(&READERS, is_wayland()), TOOL_TIMEOUT).await?)
}

async fn read_first(candidates: &[(&str, &[&str])], limit: Duration) -> anyhow::Result<String> {
    let mut timed_out = false;

    for (bin, args) in candidates {
        let mut cmd = tokio::process::Command::new(bin);
        cmd.args(*args).kill_on_drop(true);

        match tokio::time::timeout(limit, cmd.output()).await {
            Ok(Ok(out)) if out.status.success() => {
                return Ok(String::from_utf8_lossy(&out.stdout).to_string());
            }
            // Some tools exit non-zero on an empty clipboard.
            Ok(Ok(_)) => return Ok(String::new()),
            Err(_) => {
                timed_out = true;
                continue;
            }
            Ok(Err(_)) => continue,
        }
    }

    if timed_out {
        Err(anyhow::anyhow!("clipboard read timed out after {limit:?}"))
    } else {
        Err(anyhow::anyhow!(missing_tools(candidates)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLOW: (&str, &[&str]) = ("sleep", &["5"]);

    #[test]
    fn x11_session_skips_the_wayland_pair() {
        let tools = session_tools(&READERS, false);
        assert!(tools.iter().all(|(bin, _)| !bin.starts_with("wl-")));
        assert!(!tools.is_empty());
    }

    #[test]
    fn wayland_session_keeps_every_tool() {
        assert_eq!(session_tools(&WRITERS, true).len(), WRITERS.len());
    }

    #[tokio::test]
    async fn read_skips_a_reader_that_hangs() {
        let out = read_first(&[SLOW, ("echo", &["-n", "hi"])], Duration::from_millis(200)).await;
        assert_eq!(out.unwrap(), "hi");
    }

    #[tokio::test]
    async fn read_reports_a_timeout_over_a_missing_tool() {
        let out = read_first(&[SLOW], Duration::from_millis(100)).await;
        assert!(out.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn write_skips_a_writer_that_hangs() {
        let out = write_first(&[SLOW, ("cat", &[])], Duration::from_millis(200), "hi").await;
        assert!(out.is_ok());
    }

    #[tokio::test]
    async fn write_stops_at_the_first_success() {
        let out = write_first(&[("cat", &[]), SLOW], Duration::from_millis(200), "hi").await;
        assert!(out.is_ok());
    }
}
