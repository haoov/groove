//! A server scripted from the test, on the other end of an in-memory pipe.

use std::path::{Path, PathBuf};

use groove_types::Diagnostic;
use serde_json::{Value, json};
use tokio::io::{BufReader, DuplexStream, ReadHalf, WriteHalf};
use tokio::sync::{mpsc, oneshot};

use crate::{Client, Hooks, connect, frame};

pub(crate) const ROOT: &str = "/work/tree";

pub(crate) struct Fake {
    reader: BufReader<ReadHalf<DuplexStream>>,
    writer: WriteHalf<DuplexStream>,
}

pub(crate) struct Heard {
    pub(crate) diagnostics: mpsc::UnboundedReceiver<(PathBuf, Vec<Diagnostic>)>,
    pub(crate) closed: oneshot::Receiver<()>,
}

impl Fake {
    pub(crate) async fn next(&mut self) -> Value {
        frame::read(&mut self.reader).await.unwrap().unwrap()
    }

    /// The next message, which must be `method`.
    pub(crate) async fn expect(&mut self, method: &str) -> Value {
        let message = self.next().await;
        assert_eq!(message["method"], method, "{message}");
        message
    }

    pub(crate) async fn send(&mut self, message: Value) {
        frame::write(&mut self.writer, &message).await.unwrap();
    }

    pub(crate) async fn answer(&mut self, request: &Value, result: Value) {
        let id = request["id"].clone();
        self.send(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
            .await;
    }
}

/// A client and its fake server, apart: the handshake is the test's to play.
pub(crate) fn pair(
    settings: Value,
) -> (
    tokio::task::JoinHandle<groove_types::Result<Client>>,
    Fake,
    Heard,
) {
    let (near, far) = tokio::io::duplex(1 << 16);
    let (reader, writer) = tokio::io::split(near);
    let (diagnostics, heard) = mpsc::unbounded_channel();
    let (closed, gone) = oneshot::channel();
    let hooks = Hooks {
        on_diagnostics: Box::new(move |path, found| {
            let _ = diagnostics.send((path, found));
        }),
        on_closed: Box::new(move || {
            let _ = closed.send(());
        }),
    };
    let connecting = tokio::spawn(connect(reader, writer, Path::new(ROOT), settings, hooks));
    let (reader, writer) = tokio::io::split(far);
    let fake = Fake {
        reader: BufReader::new(reader),
        writer,
    };
    let heard = Heard {
        diagnostics: heard,
        closed: gone,
    };
    (connecting, fake, heard)
}

/// A client past its handshake, on a server that counts in `encoding`.
pub(crate) async fn connected(encoding: Option<&str>, settings: Value) -> (Client, Fake, Heard) {
    let (connecting, mut fake, heard) = pair(settings);
    let initialize = fake.expect("initialize").await;
    let capabilities = json!({ "capabilities": { "positionEncoding": encoding } });
    fake.answer(&initialize, capabilities).await;
    let client = connecting.await.unwrap().unwrap();
    fake.expect("initialized").await;
    fake.expect("workspace/didChangeConfiguration").await;
    (client, fake, heard)
}
