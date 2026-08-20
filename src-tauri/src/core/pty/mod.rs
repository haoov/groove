//! PTY mechanics: spawn a child on a pseudo-terminal, stream its output to the
//! frontend, accept writes/resizes, and end it. What runs in the PTY is the
//! caller's business (see `agent_manager`).

use std::{
    collections::HashMap,
    io::Write,
    sync::{Arc, Mutex},
};

use tauri::Emitter;

// SAFETY: PTY master fd (TIOCSWINSZ ioctl) is safe to call from any thread on Unix.
struct SendableMasterPty(Box<dyn portable_pty::MasterPty>);
unsafe impl Send for SendableMasterPty {}

struct Entry {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Arc<Mutex<SendableMasterPty>>,
    pid: Option<u32>,
}

/// What to run and where. No uniqueness constraint: a session opens as many
/// PTYs as it likes; each spawn returns its own id.
pub struct PtySpec {
    pub task_id: String,
    /// "agent" | "terminal" | "auth" — echoed in `pty_started` for the frontend.
    pub kind: &'static str,
    pub cwd: String,
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(&'static str, String)>,
    /// Runs once when the child exits, before `pty_exit` is emitted.
    pub on_exit: Option<Box<dyn FnOnce() + Send>>,
}

/// Registry of live PTYs, keyed by a generated session id.
pub struct Ptys {
    entries: Arc<Mutex<HashMap<String, Entry>>>,
}

impl Ptys {
    pub fn new() -> Self {
        Self { entries: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn spawn(&self, app: &tauri::AppHandle, spec: PtySpec) -> anyhow::Result<String> {
        let pair = portable_pty::native_pty_system()
            // The conventional default. The frontend resizes to the real geometry
            // as soon as the host attaches; until then a shell that wraps at 80 is
            // far less wrong than one that wraps at 120.
            .openpty(portable_pty::PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| anyhow::anyhow!("openpty failed: {e}"))?;

        let mut cmd = portable_pty::CommandBuilder::new(&spec.program);
        for arg in &spec.args {
            cmd.arg(arg);
        }
        cmd.cwd(&spec.cwd);
        describe_terminal(&mut cmd);
        for (key, value) in &spec.env {
            cmd.env(key, value);
        }

        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| anyhow::anyhow!("spawn failed: {e}"))?;
        let pid = child.process_id();

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| anyhow::anyhow!("take_writer failed: {e}"))?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| anyhow::anyhow!("clone_reader failed: {e}"))?;

        let session_id = uuid::Uuid::new_v4().to_string();

        // Background reader forwards output to the frontend. It owns the child
        // handle so it can `wait()` on natural exit (avoiding a zombie), then it
        // reaps the entry from the registry.
        let app_reader = app.clone();
        let sid = session_id.clone();
        let entries = Arc::clone(&self.entries);
        let on_exit = spec.on_exit;
        tokio::task::spawn_blocking(move || {
            let mut reader = reader;
            let mut buf = [0u8; 4096];
            loop {
                match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) | Err(_) => break,
                    // Base64, not a JSON array of numbers. This is the busiest
                    // path in the app — an agent redraws its whole screen as it
                    // thinks — and a number array costs about four JSON characters
                    // per byte, every one of them parsed individually on the UI
                    // thread. Measured on the same 2 MB of output: 14.7 KB per
                    // 4 KB chunk against 5.5 KB, and 37 ms of parsing against 9 ms.
                    Ok(n) => {
                        let _ = app_reader.emit(
                            crate::core::events::PTY_OUTPUT,
                            serde_json::json!({
                                "session_id": sid,
                                "b64": base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    &buf[..n],
                                ),
                            }),
                        );
                    }
                }
            }
            let _ = child.wait();
            if let Ok(mut map) = entries.lock() {
                map.remove(&sid);
            }
            if let Some(hook) = on_exit {
                hook();
            }
            let _ = app_reader.emit(crate::core::events::PTY_EXIT, serde_json::json!({ "session_id": sid }));
        });

        if let Ok(mut map) = self.entries.lock() {
            map.insert(
                session_id.clone(),
                Entry {
                    writer: Arc::new(Mutex::new(writer)),
                    master: Arc::new(Mutex::new(SendableMasterPty(pair.master))),
                    pid,
                },
            );
        }

        app.emit(
            crate::core::events::PTY_STARTED,
            serde_json::json!({
                "session_id": session_id,
                "task_id": spec.task_id,
                "pty_type": spec.kind,
            }),
        )?;

        Ok(session_id)
    }

    pub async fn write(&self, session_id: &str, data: Vec<u8>) -> Result<(), String> {
        let writer = self
            .entries
            .lock()
            .ok()
            .and_then(|map| map.get(session_id).map(|e| Arc::clone(&e.writer)))
            .ok_or_else(|| format!("session {session_id} not found"))?;

        // The write can block (PTY buffer full); do it off the async runtime,
        // locking the std Mutex inside the blocking closure rather than across
        // the .await.
        tokio::task::spawn_blocking(move || {
            writer
                .lock()
                .map_err(|_| "writer lock poisoned".to_string())?
                .write_all(&data)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    pub fn resize(&self, session_id: &str, rows: u16, cols: u16) -> Result<(), String> {
        let master = self
            .entries
            .lock()
            .ok()
            .and_then(|map| map.get(session_id).map(|e| Arc::clone(&e.master)))
            .ok_or_else(|| format!("session {session_id} not found"))?;
        let guard = master.lock().map_err(|_| "lock poisoned".to_string())?;
        guard
            .0
            .resize(portable_pty::PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| e.to_string())
    }

    /// SIGTERM the child and drop the entry. The reader thread sees EOF, reaps
    /// the process, and runs the exit hook; the immediate `pty_exit` below keeps
    /// the UI honest even if the child ignores the signal.
    pub fn kill(&self, app: &tauri::AppHandle, session_id: &str) -> Result<(), String> {
        if let Ok(mut map) = self.entries.lock() {
            if let Some(entry) = map.remove(session_id) {
                if let Some(pid) = entry.pid {
                    unsafe { libc::kill(pid as i32, libc::SIGTERM) };
                }
            }
        }
        app.emit(crate::core::events::PTY_EXIT, serde_json::json!({ "session_id": session_id }))
            .map_err(|e| e.to_string())
    }
}

impl Default for Ptys {
    fn default() -> Self {
        Self::new()
    }
}

/// Tell the child what terminal it is talking to.
///
/// The frontend is xterm.js, so this is the truth and never the launcher's idea
/// of it: a desktop launch has no `TERM` at all, and a program with no terminfo
/// cannot move the cursor. Inheriting `TERM` is just as wrong: a shell told it
/// is inside tmux writes sequences xterm.js does not implement.
fn describe_terminal(cmd: &mut portable_pty::CommandBuilder) {
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
}

// ─── IPC ──────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn stop_agent_session(
    app: tauri::AppHandle,
    session_id: String,
    ptys: tauri::State<'_, Ptys>,
) -> Result<(), String> {
    ptys.kill(&app, &session_id)
}

#[tauri::command]
pub async fn write_pty(
    session_id: String,
    // Base64, not Vec<u8>: a JSON number array costs four characters and a parse
    // per byte on the hottest IPC path (every keystroke). Symmetric with pty_output.
    data_b64: String,
    ptys: tauri::State<'_, Ptys>,
) -> Result<(), String> {
    let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &data_b64)
        .map_err(|e| format!("bad base64 pty payload: {e}"))?;
    ptys.write(&session_id, data).await
}

#[tauri::command]
pub async fn resize_pty(
    session_id: String,
    rows: u16,
    cols: u16,
    ptys: tauri::State<'_, Ptys>,
) -> Result<(), String> {
    ptys.resize(&session_id, rows, cols)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A PTY child with no TERM cannot move its cursor, and a shell's line redraw
    // then duplicates characters. The value has to be one xterm.js actually
    // implements, and it has to be SET rather than inherited.
    #[test]
    fn pty_children_are_told_they_are_an_xterm() {
        let mut cmd = portable_pty::CommandBuilder::new("bash");
        describe_terminal(&mut cmd);
        assert_eq!(cmd.get_env("TERM").unwrap(), "xterm-256color");
        assert_eq!(cmd.get_env("COLORTERM").unwrap(), "truecolor");
    }
}
