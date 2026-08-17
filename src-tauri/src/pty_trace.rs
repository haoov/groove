//! Byte-level PTY tracing, off unless a marker file exists.
//!
//! Four tags, so a duplicated character can be attributed to one hop instead of
//! guessed at: `js>>` a keystroke leaving xterm, `pty>>` the same bytes reaching
//! the shell, `pty<<` the shell's answer, `js<<` that answer reaching xterm.
//! A character that appears once as `pty<<` and twice as `js<<` is an event
//! delivery bug; twice at both is the shell; once at both is the renderer.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static SINK: OnceLock<Option<Mutex<File>>> = OnceLock::new();

/// Open the log when `<data dir>/pty-trace.on` exists. Called once at startup;
/// creating the marker later takes effect on the next launch.
pub fn init(data_dir: &Path) {
    let _ = SINK.set(if data_dir.join("pty-trace.on").exists() {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(data_dir.join("pty-trace.log"))
            .ok()
            .map(Mutex::new)
    } else {
        None
    });
}

/// Whether the webview should bother reporting its two hops.
#[tauri::command]
pub fn pty_trace_on() -> bool {
    matches!(SINK.get(), Some(Some(_)))
}

/// Append one line: time, tag, session, byte count, and the bytes as text.
pub fn record(tag: &str, session_id: &str, data: &[u8]) {
    let Some(Some(sink)) = SINK.get() else { return };
    let Ok(mut file) = sink.lock() else { return };
    let short = session_id.get(..8).unwrap_or(session_id);
    let stamp = chrono::Utc::now().format("%H:%M:%S%.3f");
    let _ = writeln!(
        file,
        "{stamp} {tag} {short} {:>4} {:?}",
        data.len(),
        String::from_utf8_lossy(data)
    );
}

/// The webview's end of the same PTY, so both hops land in one ordered file.
#[tauri::command]
pub fn trace_pty(tag: String, session_id: String, data: Vec<u8>) {
    record(&tag, &session_id, &data);
}
