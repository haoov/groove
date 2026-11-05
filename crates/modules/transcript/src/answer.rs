//! What an answer of the agent's says beside its words: the API error it stands for, what it spent.

use groove_types::chat::Heard;
use serde_json::Value;

/// An API error the agent's message stands for, by its text.
pub(crate) fn failed(content: Option<&Value>) -> Option<Heard> {
    let texts = content?.as_array()?.iter();
    let texts: Vec<&str> = texts.filter_map(|one| one.get("text")?.as_str()).collect();
    let text = texts.join("\n");
    (!text.trim().is_empty()).then(|| Heard::Failed(text.trim().to_string()))
}

/// The model that wrote an answer, and the tokens its context held for it.
pub(crate) fn spent(record: &Value) -> Option<Heard> {
    let message = record.get("message")?;
    let usage = message.get("usage")?;
    let tokens = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or_default();
    let context = tokens("input_tokens")
        + tokens("cache_creation_input_tokens")
        + tokens("cache_read_input_tokens");
    let model = message.get("model")?.as_str()?.to_string();
    Some(Heard::Used { model, context })
}
