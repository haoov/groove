//! What the server sends: answers to settle, requests to answer, notices to hand on.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use groove_types::Diagnostic;
use serde_json::{Value, json};
use tokio::io::{AsyncRead, BufReader};

use crate::documents::Documents;
use crate::link::{Link, lock};
use crate::{frame, protocol, uri};

/// What a client hears from its server, called off the main thread.
pub struct Hooks {
    /// A file's whole set of diagnostics, replacing the last one.
    pub on_diagnostics: Box<dyn Fn(PathBuf, Vec<Diagnostic>) + Send + Sync>,
    /// The server stopped talking: it exited, or closed its output.
    pub on_closed: Box<dyn FnOnce() + Send>,
}

pub(crate) struct Incoming {
    pub(crate) link: Arc<Link>,
    pub(crate) documents: Arc<Mutex<Documents>>,
    pub(crate) settings: Value,
    pub(crate) root: PathBuf,
    pub(crate) hooks: Hooks,
}

impl Incoming {
    pub(crate) async fn run<R: AsyncRead + Unpin>(self, reader: R) {
        let mut reader = BufReader::new(reader);
        while let Ok(Some(message)) = frame::read(&mut reader).await {
            self.dispatch(message);
        }
        self.link.close();
        (self.hooks.on_closed)();
    }

    fn dispatch(&self, message: Value) {
        let method = message.get("method").and_then(Value::as_str);
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        match (method, message.get("id").cloned()) {
            (None, Some(id)) => self.settle(&message, id),
            (Some(method), Some(id)) => self.answer(method, id, &params),
            (Some(method), None) => self.hear(method, &params),
            (None, None) => {}
        }
    }

    fn settle(&self, message: &Value, id: Value) {
        let Some(id) = id.as_i64() else {
            return;
        };
        let answer = match message.get("error") {
            Some(error) => Err(error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("an error without a message")
                .to_string()),
            None => Ok(message.get("result").cloned().unwrap_or(Value::Null)),
        };
        self.link.settle(id, answer);
    }

    fn answer(&self, method: &str, id: Value, params: &Value) {
        let result = match method {
            "workspace/configuration" => self.configuration(params),
            "workspace/workspaceFolders" => {
                json!([{ "uri": uri::of(&self.root), "name": self.root.to_string_lossy() }])
            }
            "client/registerCapability"
            | "client/unregisterCapability"
            | "window/workDoneProgress/create"
            | "window/showMessageRequest" => Value::Null,
            _ => {
                let _ = self
                    .link
                    .refuse(id, -32601, &format!("{method} is not handled"));
                return;
            }
        };
        let _ = self.link.reply(id, result);
    }

    fn configuration(&self, params: &Value) -> Value {
        let items = params.get("items").and_then(Value::as_array);
        let section = |item: &Value| {
            item.get("section")
                .and_then(Value::as_str)
                .map(String::from)
        };
        let sections = items.map(|items| items.iter().map(section).collect::<Vec<_>>());
        let read = sections.unwrap_or_default().into_iter();
        Value::Array(
            read.map(|s| protocol::section(&self.settings, s.as_deref()))
                .collect(),
        )
    }

    fn hear(&self, method: &str, params: &Value) {
        if method != "textDocument/publishDiagnostics" {
            return;
        }
        let read = protocol::diagnostics(params, &lock(&self.documents));
        if let Some((path, found)) = read {
            (self.hooks.on_diagnostics)(path, found);
        }
    }
}
