//! What the agent asks before a tool runs: leave to run it, or the answers to its questions.

use serde_json::Value;

/// A tool call waiting on the user's leave.
#[derive(Debug, Clone, PartialEq)]
pub struct Permission {
    /// The request id the answer goes back under.
    pub id: String,
    pub tool: String,
    pub detail: Option<String>,
    /// The questions an `AskUserQuestion` call puts; empty for any other tool.
    pub questions: Vec<Question>,
    /// The call's input, handed back with the leave.
    pub input: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub question: String,
    pub header: String,
    pub options: Vec<String>,
}

impl Permission {
    /// The input with `picked` as the answer to each question, in order.
    pub fn answered(&self, picked: &[String]) -> Value {
        let answers = self.questions.iter().zip(picked);
        let answers =
            answers.map(|(one, label)| (one.question.clone(), Value::from(label.as_str())));
        let mut input = self.input.clone();
        if let Some(fields) = input.as_object_mut() {
            fields.insert("answers".into(), Value::Object(answers.collect()));
        }
        input
    }
}
