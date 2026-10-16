//! One server, started or connected, and what Groove asks of it.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use groove_exec::piped::{Piped, Pipes, Process};
use groove_types::{Caret, Error, ErrorKind, Location, Result};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, BufReader};
use tokio::sync::mpsc;

use crate::documents::Documents;
use crate::incoming::{Hooks, Incoming};
use crate::link::{Link, lock};
use crate::position::Encoding;
use crate::{frame, protocol, uri};

/// How many lines of the server's log a client keeps.
const SAID: usize = 100;
/// How long a server has to exit once asked.
const EXIT: Duration = Duration::from_secs(2);

/// How to start a server, and the settings it reads.
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    /// Variables the server does not inherit, as `VIRTUAL_ENV`.
    pub unset: Vec<String>,
    pub root: PathBuf,
    pub settings: Value,
}

pub struct Client {
    link: Arc<Link>,
    documents: Arc<Mutex<Documents>>,
    process: Option<Process>,
    said: Arc<Mutex<VecDeque<String>>>,
}

/// Starts the server `launch` names in its root, and initializes it.
pub async fn start(launch: Launch, hooks: Hooks) -> Result<Client> {
    let mut piped = Piped::new(&launch.program)
        .args(launch.args.iter().cloned())
        .cwd(&launch.root);
    for key in &launch.unset {
        piped = piped.unset(key);
    }
    let mut process = piped
        .start()
        .map_err(|e| Error::new(ErrorKind::Io, e.to_string()))?;
    let Some(Pipes {
        stdin,
        stdout,
        stderr,
    }) = process.pipes()
    else {
        return Err(Error::internal("the server's pipes were taken"));
    };
    let said = Arc::new(Mutex::new(VecDeque::new()));
    tokio::spawn(drain(stderr, said.clone()));
    let mut client = connect(stdout, stdin, &launch.root, launch.settings, hooks).await?;
    client.process = Some(process);
    client.said = said;
    Ok(client)
}

/// Initializes a server already reachable through `reader` and `writer`.
pub async fn connect<R, W>(
    reader: R,
    mut writer: W,
    root: &Path,
    settings: Value,
    hooks: Hooks,
) -> Result<Client>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (outgoing, mut queued) = mpsc::unbounded_channel::<Value>();
    tokio::spawn(async move {
        while let Some(message) = queued.recv().await {
            if frame::write(&mut writer, &message).await.is_err() {
                break;
            }
        }
    });
    let link = Arc::new(Link::new(outgoing));
    let documents = Arc::new(Mutex::new(Documents::default()));
    let incoming = Incoming {
        link: link.clone(),
        documents: documents.clone(),
        settings: settings.clone(),
        root: root.to_path_buf(),
        hooks,
    };
    tokio::spawn(incoming.run(reader));
    let answer = link
        .request("initialize", protocol::initialize(root))
        .await?;
    let named = answer
        .pointer("/capabilities/positionEncoding")
        .and_then(Value::as_str);
    lock(&documents).encoding = Encoding::named(named);
    link.notify("initialized", json!({}))?;
    link.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": settings }),
    )?;
    Ok(Client {
        link,
        documents,
        process: None,
        said: Arc::default(),
    })
}

impl Client {
    pub fn encoding(&self) -> Encoding {
        lock(&self.documents).encoding
    }

    /// The last lines the server wrote to its log.
    pub fn said(&self) -> Vec<String> {
        lock(&self.said).iter().cloned().collect()
    }

    pub async fn open(&self, path: &Path, language: &str, text: String) -> Result<()> {
        let document =
            json!({ "uri": uri::of(path), "languageId": language, "version": 1, "text": text });
        lock(&self.documents).open(path, text);
        self.link
            .notify("textDocument/didOpen", json!({ "textDocument": document }))
    }

    /// Sends the whole new text of an open document.
    pub async fn change(&self, path: &Path, text: String) -> Result<()> {
        let Some(version) = lock(&self.documents).change(path, text.clone()) else {
            return Err(Error::invalid(format!("{} is not open", path.display())));
        };
        let document = json!({ "uri": uri::of(path), "version": version });
        let changes = json!([{ "text": text }]);
        let params = json!({ "textDocument": document, "contentChanges": changes });
        self.link.notify("textDocument/didChange", params)
    }

    pub async fn save(&self, path: &Path) -> Result<()> {
        let params = json!({ "textDocument": { "uri": uri::of(path) } });
        self.link.notify("textDocument/didSave", params)
    }

    pub async fn close(&self, path: &Path) -> Result<()> {
        if !lock(&self.documents).close(path) {
            return Ok(());
        }
        let params = json!({ "textDocument": { "uri": uri::of(path) } });
        self.link.notify("textDocument/didClose", params)
    }

    pub async fn definition(&self, path: &Path, at: Caret) -> Result<Vec<Location>> {
        let answer = self.ask("textDocument/definition", path, at).await?;
        Ok(protocol::locations(&answer, &lock(&self.documents)))
    }

    /// What the server says of the symbol at `at`, as Markdown.
    pub async fn hover(&self, path: &Path, at: Caret) -> Result<Option<String>> {
        let answer = self.ask("textDocument/hover", path, at).await?;
        Ok(protocol::hover(&answer))
    }

    /// Asks the server to exit, then waits a moment; dropping the client kills what is left.
    pub async fn shutdown(mut self) {
        if self.link.request("shutdown", Value::Null).await.is_ok() {
            let _ = self.link.notify("exit", Value::Null);
        }
        if let Some(process) = self.process.as_mut() {
            let _ = tokio::time::timeout(EXIT, process.wait()).await;
        }
    }

    async fn ask(&self, method: &str, path: &Path, at: Caret) -> Result<Value> {
        let position = {
            let documents = lock(&self.documents);
            if !documents.is_open(path) {
                return Err(Error::invalid(format!("{} is not open", path.display())));
            }
            documents.position(path, at)
        };
        let document = json!({ "uri": uri::of(path) });
        let params = json!({ "textDocument": document, "position": position });
        self.link.request(method, params).await
    }
}

async fn drain<R: AsyncRead + Unpin>(stderr: R, said: Arc<Mutex<VecDeque<String>>>) {
    let mut lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let mut kept = lock(&said);
        if kept.len() == SAID {
            kept.pop_front();
        }
        kept.push_back(line);
    }
}
