//! System clipboard, through the OS rather than the webview.
//!
//! Tauri's Linux webview is WebKitGTK, where `navigator.clipboard` is missing
//! unless the origin counts as a secure context, and `document.execCommand('copy')`
//! needs a real DOM selection — which a terminal does not have: xterm draws its own
//! selection, so there is nothing for the browser to copy. Both webview routes
//! failed silently, which is worse than failing loudly.
//!
//! Shelling out to the clipboard tool is deliberate over pulling in `arboard`: it
//! is what actually works on this desktop today (verified: `xclip` round-trips
//! under XWayland), it adds no dependency, and Wayland-native support is a matter
//! of installing `wl-clipboard` rather than rebuilding.

use tokio::io::AsyncWriteExt;

/// Writers, in preference order: Wayland-native first, then X11 (which XWayland
/// bridges), then the older xsel.
const WRITERS: [(&str, &[&str]); 3] = [
    ("wl-copy", &[]),
    ("xclip", &["-selection", "clipboard"]),
    ("xsel", &["--clipboard", "--input"]),
];

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

/// Put `text` on the system clipboard — on EVERY clipboard the desktop has.
///
/// Verified on this desktop: the Wayland and X11 clipboards are entirely separate
/// (a `wl-copy` is invisible to `xclip` and the reverse), with no bridging in either
/// direction. Stopping at the first tool that succeeded therefore filled one
/// clipboard and left the other holding something stale — which reads as "copy does
/// nothing" whenever the paste target happens to use the other one.
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

        // Not installed — not an error, the desktop simply has no such clipboard.
        let Ok(mut child) = spawned else { continue };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(text.as_bytes()).await {
                errors.push(format!("{bin}: {e}"));
                continue;
            }
            // The tool copies until stdin closes; without this it waits for ever.
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
            // An empty clipboard makes some tools exit non-zero; that is not an
            // error worth failing over, and no other tool would do better.
            Ok(_) => return Ok(String::new()),
            Err(_) => continue,
        }
    }
    Err(missing_tools(&READERS))
}
