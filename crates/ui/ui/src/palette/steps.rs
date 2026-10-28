//! A flow's steps: an answer taken, taken back, or the flow left.

use groove_controllers::AppState;

use super::{Act, Action, Outcome, Palette};

impl Palette {
    /// The typed text or the picked option becomes the answer; the flow moves on or ends.
    pub(super) fn answer(&mut self, app: &AppState) -> Outcome {
        let Some(prompt) = self.prompt(app) else {
            return Outcome::default();
        };
        let picked = self.options(app).into_iter().nth(self.selected);
        let (said, value) = match picked {
            Some(pick)
                if !prompt.options.is_empty() && !(self.query.is_empty() && prompt.allow_empty) =>
            {
                pick
            }
            _ if prompt.free || (prompt.allow_empty && self.query.is_empty()) => {
                let typed = self.query.text().to_string();
                (typed.clone(), typed)
            }
            _ => return Outcome::default(),
        };
        if value.is_empty() && !prompt.allow_empty {
            return Outcome::default();
        }
        let Some(flow) = self.flow.as_mut() else {
            return Outcome::default();
        };
        flow.answer(value, said);
        self.reset();
        let Some(flow) = self.flow.as_ref() else {
            return Outcome::default();
        };
        if flow.prompt(app).is_some() {
            return Outcome {
                commands: flow.refresh(app).into_iter().collect(),
                ..Outcome::default()
            };
        }
        self.ended()
    }

    /// Every answer in: the flow's command, or the kind it picked; the palette shuts.
    fn ended(&mut self) -> Outcome {
        let Some(flow) = self.flow.take() else {
            return Outcome::default();
        };
        let (commands, act) = match flow.action {
            Action::PickKind => (Vec::new(), flow.answers.first().cloned().map(Act::Kind)),
            _ => (flow.command().into_iter().collect(), None),
        };
        Outcome {
            commands,
            act,
            close: true,
            ..Outcome::default()
        }
    }

    /// Back one prompt; out of the flow; closed.
    pub(super) fn escape(&mut self) -> Outcome {
        match self.flow.as_mut() {
            Some(flow) if !flow.answers.is_empty() => {
                flow.unanswer();
                self.reset();
                Outcome::default()
            }
            Some(flow) if !flow.from.0.is_empty() => {
                self.flow = None;
                self.reset();
                Outcome::default()
            }
            _ => Outcome {
                close: true,
                ..Outcome::default()
            },
        }
    }

    /// Backspace on an empty query: the last answer back in the field, else the entries again.
    pub(super) fn back(&mut self) {
        let Some(flow) = self.flow.as_mut() else {
            return;
        };
        if let Some(said) = flow.unanswer() {
            self.reset();
            self.query.set(&said);
            return;
        }
        if flow.from.0.is_empty() {
            return;
        }
        let query = std::mem::take(&mut flow.from.1);
        self.flow = None;
        self.reset();
        self.query.set(&query);
    }
}
