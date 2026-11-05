//! What the agent's last answer used: its model, and how full its context stood.

use super::{Chat, Limits};

impl Chat {
    /// The agent's words as it writes them.
    pub fn writing(&self) -> &str {
        &self.writing
    }

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

/// `opus-5-5 · context 82k · 5h 48% · week 9%`, leaving out what is not known.
pub fn usage(model: Option<&str>, context: Option<u64>, limits: Limits) -> String {
    let model = model.map(|one| one.trim_start_matches("claude-").to_string());
    let context = context.map(|one| format!("context {}k", one / 1000));
    let share = |word: &str, used: Option<f32>| {
        used.map(|one| format!("{word} {}%", (one * 100.0).round() as u32))
    };
    let five = share("5h", limits.five_hour);
    let week = share("week", limits.week);
    let parts = [model, context, five, week].into_iter().flatten();
    parts.collect::<Vec<_>>().join(" · ")
}
