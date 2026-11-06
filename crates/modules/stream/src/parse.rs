//! One line of the agent's output read into what Groove hears; a kind the transcript already says is skipped.

use groove_types::Timestamp;
use groove_types::chat::{self, Permission, Question};
use serde_json::{Value, json};

use crate::Heard;

/// What the line says, or the refusal of a request Groove does not serve.
pub(crate) fn line(text: &str) -> Option<Result<Heard, Value>> {
    let record = serde_json::from_str::<Value>(text).ok()?;
    let id = || Some(record.get("request_id")?.as_str()?.to_string());
    match record.get("type")?.as_str()? {
        "result" => Some(Ok(settled(&record))),
        "control_cancel_request" => Some(Ok(Heard::Withdrawn(id()?))),
        "control_request" => Some(asks(&record).ok_or_else(|| refused(&id().unwrap_or_default()))),
        "assistant" | "user" => within(&record, text).map(Ok),
        "rate_limit_event" => windows(&record).map(Ok),
        "system" => commands(&record).map(Ok),
        _ => None,
    }
}

/// A subagent's own record, by the call that sent it; the main agent's is the transcript's to say.
fn within(record: &Value, text: &str) -> Option<Heard> {
    let parent = record.get("parent_tool_use_id")?.as_str()?.to_string();
    let heard = groove_transcript::line(text);
    let heard = chat::Heard::Within { parent, heard };
    Some(Heard::Chat(heard))
}

/// The commands its prompt takes, each time they change.
fn commands(record: &Value) -> Option<Heard> {
    if record.get("subtype")?.as_str()? != "commands_changed" {
        return None;
    }
    let text = |one: &Value, key: &str| one.get(key).and_then(Value::as_str).map(str::to_string);
    let listed = record.get("commands")?.as_array()?.iter();
    let slashes = listed.filter_map(|one| {
        let about = text(one, "description").unwrap_or_default();
        Some(chat::Slash {
            name: text(one, "name")?,
            about,
        })
    });
    Some(Heard::Commands(slashes.collect()))
}

/// A limit that refuses the next request, and when it lifts.
fn limited(info: &Value) -> Option<String> {
    if info.get("status")?.as_str()? != "rejected" {
        return None;
    }
    let kind = info.get("rateLimitType").and_then(Value::as_str);
    let kind = kind.unwrap_or("usage").replace('_', "-");
    let resets = info.get("resetsAt").and_then(Value::as_i64);
    let wait = resets.map(|at| Timestamp::now().age_at(Timestamp::new(at)).as_secs() / 60);
    let text = match wait {
        Some(minutes) => format!(
            "The {kind} limit is reached; it resets in {}h {:02}m.",
            minutes / 60,
            minutes % 60
        ),
        None => format!("The {kind} limit is reached."),
    };
    Some(text)
}

/// How much of the plan's windows is used, and the limit that refuses the next request if one does.
fn windows(record: &Value) -> Option<Heard> {
    let info = record.get("rate_limit_info")?;
    let used = |window: &str| {
        let held = info.pointer(&format!("/unifiedWindows/{window}"))?;
        let used = held.get("utilization")?.as_f64()? as f32;
        let resets = held
            .get("resetsAt")
            .and_then(Value::as_i64)
            .map(Timestamp::new);
        Some(chat::Window { used, resets })
    };
    let limits = chat::Limits {
        five_hour: used("five_hour"),
        week: used("seven_day"),
    };
    let refused = limited(info);
    Some(Heard::Limits { limits, refused })
}

/// A turn's end, with how many tokens each model it used can hold.
fn settled(record: &Value) -> Heard {
    let used = record.get("modelUsage").and_then(Value::as_object);
    let windows = used.into_iter().flatten().filter_map(|(model, one)| {
        let window = one.get("contextWindow")?.as_u64()?;
        Some((model.clone(), window))
    });
    Heard::Settled {
        windows: windows.collect(),
    }
}

/// A `can_use_tool` request as the permission it waits on.
fn asks(record: &Value) -> Option<Heard> {
    let request = record.get("request")?;
    if request.get("subtype")?.as_str()? != "can_use_tool" {
        return None;
    }
    let input = request.get("input").cloned().unwrap_or_else(|| json!({}));
    Some(Heard::Asks(Permission {
        id: record.get("request_id")?.as_str()?.to_string(),
        tool: request.get("tool_name")?.as_str()?.to_string(),
        detail: groove_transcript::detail(Some(&input)),
        questions: questions(&input),
        input,
    }))
}

fn questions(input: &Value) -> Vec<Question> {
    let asked = input.get("questions").and_then(Value::as_array);
    let text = |one: &Value, key: &str| {
        let found = one.get(key).and_then(Value::as_str);
        found.unwrap_or_default().to_string()
    };
    let options = |one: &Value| {
        let all = one.get("options").and_then(Value::as_array);
        all.into_iter()
            .flatten()
            .map(|option| text(option, "label"))
            .collect()
    };
    let asked = asked.into_iter().flatten();
    asked
        .map(|one| Question {
            question: text(one, "question"),
            header: text(one, "header"),
            options: options(one),
        })
        .collect()
}

pub(crate) fn success(id: &str, response: Value) -> Value {
    json!({
        "type": "control_response",
        "response": { "subtype": "success", "request_id": id, "response": response },
    })
}

fn refused(id: &str) -> Value {
    json!({
        "type": "control_response",
        "response": { "subtype": "error", "request_id": id, "error": "Groove does not serve this request" },
    })
}
