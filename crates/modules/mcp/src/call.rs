//! One tool call on its way to the app, and the answer on its way back.

use groove_tools::Arguments;
use tokio::sync::oneshot;

/// A tool the agent of one session asks for, and where its answer goes.
#[derive(Debug)]
pub struct Call {
    pub session: String,
    pub tool: String,
    pub arguments: serde_json::Value,
    pub reply: Reply,
}

impl Call {
    /// One argument, as a string.
    pub fn text(&self, name: &str) -> Option<&str> {
        self.arguments.text(name)
    }

    pub fn number(&self, name: &str) -> Option<i64> {
        self.arguments.number(name)
    }

    pub fn flag(&self, name: &str) -> Option<bool> {
        self.arguments.flag(name)
    }
}

/// What a tool answers: the text the agent reads, and whether it failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub text: String,
    pub failed: bool,
}

impl Answer {
    pub fn said(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            failed: false,
        }
    }

    pub fn failed(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            failed: true,
        }
    }

    /// What a tool that answers in data says.
    pub fn json(value: &serde_json::Value) -> Self {
        Self::said(serde_json::to_string_pretty(value).unwrap_or_default())
    }
}

/// The one answer a call takes. Dropping it fails the call.
#[derive(Debug)]
pub struct Reply(oneshot::Sender<Answer>);

impl Reply {
    /// The reply, and the end its answer arrives on.
    pub fn new() -> (Self, oneshot::Receiver<Answer>) {
        let (sender, answered) = oneshot::channel();
        (Self(sender), answered)
    }

    pub fn answer(self, answer: Answer) {
        let _ = self.0.send(answer);
    }

    pub fn said(self, text: impl Into<String>) {
        self.answer(Answer::said(text));
    }

    pub fn failed(self, text: impl Into<String>) {
        self.answer(Answer::failed(text));
    }

    pub fn json(self, value: &serde_json::Value) {
        self.answer(Answer::json(value));
    }
}
