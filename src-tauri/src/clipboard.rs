//! System clipboard, through the OS rather than the webview.
//!
//! `navigator.clipboard` needs a secure context and `execCommand('copy')` needs a DOM
//! selection, which a terminal does not have — xterm draws its own. Both failed
//! silently.

use tokio::io::AsyncWriteExt;

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

fn missing_tools(candidates: &[(&str, &[&str])]) -> String {
    let names: Vec<&str> = candidates.iter().map(|(bin, _)| *bin).collect();
    format!(
        "no clipboard tool found — install one of: {}",
        names.join(", ")
    )
}

/// Put `text` on EVERY clipboard the desktop has.
///
/// Do not stop at the first success: the Wayland and X11 clipboards are separate and
/// unbridged, so writing one leaves the other stale.

#[tauri::command]
pub async fn copy_to_clipboard(text: String) -> Result<(), String> {
    let mut wrote = 0usize;
    let mut errors: Vec<String> = vec![];

    for (bin, args) in WRITERS {
        let spawned = tokio::process::Command::new(bin)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        // Not installed is not an error.
        let Ok(mut child) = spawned else { continue };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(text.as_bytes()).await {
                errors.push(format!("{bin}: {e}"));
                continue;
            }
            // The tool copies until stdin closes.
            drop(stdin);
        }

        match child.wait().await {
            Ok(status) if status.success() => wrote += 1,
            Ok(status) => errors.push(format!("{bin} exited with {status}")),
            Err(e) => errors.push(format!("{bin}: {e}")),
        }
    }

    if wrote > 0 {
        return Ok(());
    }
    Err(if errors.is_empty() {
        missing_tools(&WRITERS)
    } else {
        errors.join("; ")
    })
}

/// Read the system clipboard. Empty string when it holds nothing usable.
#[tauri::command]
pub async fn read_clipboard() -> Result<String, String> {
    for (bin, args) in READERS {
        match tokio::process::Command::new(bin).args(args).output().await {
            Ok(out) if out.status.success() => {
                return Ok(String::from_utf8_lossy(&out.stdout).to_string());
            }
            // Some tools exit non-zero on an empty clipboard.
            Ok(_) => return Ok(String::new()),
            Err(_) => continue,
        }
    }
    Err(missing_tools(&READERS))
}
