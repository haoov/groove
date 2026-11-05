//! The agents the agent sends: each one's card, its own steps folded in, and its answer.

use super::rows::{Glance, Run, Sent, Subagent};
use super::{Chat, Entry, Heard, Outcome, first_line};
use crate::Timestamp;

impl Chat {
    pub(super) fn sent_agent(&mut self, id: String, kind: String, task: String) -> Entry {
        Entry::Subagent(Subagent {
            id,
            kind,
            task,
            steps: Glance::default(),
            answer: None,
            state: Sent::Working,
            agent: None,
            since: self.now,
            until: self.now,
        })
    }

    /// The subagent `parent`'s own records, folded into its steps.
    fn within(&mut self, parent: &str, heard: Vec<Heard>) {
        if let Some(sub) = self.sent_mut(|sub| sub.id == parent) {
            heard.into_iter().for_each(|one| stepped(sub, one));
        }
    }

    /// Whether a subagent still works.
    pub fn sending(&self) -> bool {
        let working =
            |one: &Entry| matches!(one, Entry::Subagent(sub) if sub.state == Sent::Working);
        self.entries.iter().any(working)
    }

    /// A record about a subagent already sent: its own steps, its id in the background, or its end.
    pub(super) fn followed_agent(&mut self, heard: Heard) {
        match heard {
            Heard::Within { parent, heard } => self.within(&parent, heard),
            Heard::Background { id, agent } => self.backgrounded(&id, agent),
            Heard::Ended {
                call,
                agent,
                state,
                summary,
            } => self.ended((call.as_deref(), &agent), state, summary),
            Heard::Reported { agent, report } => {
                if let Some(sub) = self.sent_mut(|sub| sub.agent.as_deref() == Some(&agent)) {
                    sub.answer = Some(report);
                }
            }
            _ => {}
        }
    }

    fn backgrounded(&mut self, id: &str, agent: String) {
        if let Some(sub) = self.sent_mut(|sub| sub.id == id) {
            sub.agent = Some(agent);
        }
    }

    /// A background subagent's end: how it stands, and what it said unless its report did.
    fn ended(&mut self, (call, agent): (Option<&str>, &str), state: Sent, summary: Option<String>) {
        let now = self.now;
        let named =
            |sub: &Subagent| Some(sub.id.as_str()) == call || sub.agent.as_deref() == Some(agent);
        if let Some(sub) = self.sent_mut(named) {
            (sub.state, sub.answer) = (state, sub.answer.take().or(summary));
            sub.until = now.or(sub.until);
        }
    }

    fn sent_mut(&mut self, named: impl Fn(&Subagent) -> bool) -> Option<&mut Subagent> {
        self.entries.iter_mut().rev().find_map(|one| match one {
            Entry::Subagent(sub) if named(sub) => Some(sub),
            _ => None,
        })
    }

    /// The turn interrupted, and every subagent still working with it.
    pub(super) fn stopped(&mut self) -> Entry {
        for one in &mut self.entries {
            if let Entry::Subagent(sub) = one
                && sub.state == Sent::Working
            {
                sub.state = Sent::Stopped;
            }
        }
        Entry::Interrupted
    }
}

/// One record of a subagent's own: a step counted, and what it does now.
fn stepped(sub: &mut Subagent, heard: Heard) {
    let steps = &mut sub.steps;
    match heard {
        Heard::Clock(at) => sub.until = Some(at),
        Heard::Thought => steps.thoughts += 1,
        Heard::Look(at) => {
            steps.doing = Some(format!("read {at}"));
            steps.looked.push(at);
        }
        Heard::Ran { id, command, about } => {
            steps.doing = Some(about.unwrap_or_else(|| first_line(&command)));
            steps.ran.push(run(id, None, command));
        }
        Heard::Tool { id, name, detail } => {
            steps.doing = Some(name.clone());
            steps
                .ran
                .push(run(id, Some(name), detail.unwrap_or_default()));
        }
        Heard::Edit { path, .. } => steps.doing = Some(format!("edit {path}")),
        Heard::Result { id, outcome } => {
            if let Some(one) = steps.ran.iter_mut().find(|one| one.id == id) {
                (one.output, one.failed) = (outcome.output, outcome.failed);
            }
        }
        _ => {}
    }
}

fn run(id: String, tool: Option<String>, command: String) -> Run {
    Run {
        id,
        tool,
        command,
        output: None,
        failed: false,
    }
}

/// Its answer came back: done with what it reported, or failed.
pub(super) fn answered(sub: &mut Subagent, outcome: Outcome, now: Option<Timestamp>) {
    sub.state = match outcome.failed {
        true => Sent::Failed,
        false => Sent::Done,
    };
    sub.answer = outcome.output;
    sub.until = now.or(sub.until);
}
