//! What the agent's last answer used: its model, and how full its context stood.

use super::Chat;

impl Chat {
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// The tokens the context held at the last answer.
    pub fn context(&self) -> Option<u64> {
        self.context
    }

    pub(super) fn used(&mut self, model: String, context: u64) {
        (self.model, self.context) = (Some(model), Some(context));
    }
}

/// The models an agent can be switched to.
pub const MODELS: [&str; 4] = [
    "claude-opus-5-5",
    "claude-sonnet-5-5",
    "claude-haiku-5-5",
    "claude-fable-5-1",
];

/// `opus 5.5` for `claude-opus-5-5`.
pub fn model_name(id: &str) -> String {
    let id = id.trim_start_matches("claude-");
    match id.split_once('-') {
        Some((name, version)) => format!("{name} {}", version.replace('-', ".")),
        None => id.to_string(),
    }
}

/// `82k`, `1M`, `1.5M`: tokens as a gauge says them.
pub fn tokens(count: u64) -> String {
    match count {
        0..1_000 => count.to_string(),
        1_000..1_000_000 => format!("{}k", count / 1_000),
        _ => {
            let millions = format!("{:.1}", count as f64 / 1_000_000.0);
            format!("{}M", millions.trim_end_matches(".0"))
        }
    }
}

/// `resets in 1h 42m`, or in days and hours past a day.
pub fn resets(at: crate::Timestamp, now: crate::Timestamp) -> String {
    let minutes = now.age_at(at).as_secs() / 60;
    let (days, hours) = (minutes / (24 * 60), minutes / 60 % 24);
    match days {
        0 => format!("resets in {hours}h {:02}m", minutes % 60),
        days => format!("resets in {days}d {hours}h"),
    }
}

/// `48%` for a share of 0.48.
pub fn percent(share: f32) -> String {
    format!("{}%", (share * 100.0).round() as u32)
}
