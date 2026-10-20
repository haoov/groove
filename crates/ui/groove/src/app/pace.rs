//! When the window wakes on its own: how long it may sleep, and the clocks it keeps.

use std::time::Duration;

use groove_types::AttentionClass;

use super::App;
use super::events::{self, CLOCK_S};

impl App {
    /// How long the window may sleep before it redraws itself.
    pub(super) fn pace(&self) -> Option<Duration> {
        let moving = !self.state.pending.is_empty()
            || self
                .state
                .agent
                .agents
                .iter()
                .any(|(_, agent)| agent.activity.class() == AttentionClass::Moving)
            || groove_ui::spins(&self.state, &self.ui);
        if moving {
            return Some(Duration::from_millis(groove_ui::SPINNER_MS));
        }
        let tick = self.started.elapsed().as_millis() as u64;
        if let Some(left) = groove_ui::input::resting(&self.ui, tick) {
            return Some(Duration::from_millis(left));
        }
        let waiting =
            !self.state.agent.agents.is_empty() || groove_controllers::delivery::polls(&self.state);
        let clock = waiting.then(|| Duration::from_secs(CLOCK_S));
        let aging = groove_ui::ages_due(&self.state, &self.ui).map(events::until);
        clock.into_iter().chain(aging).min()
    }

    /// The clock on the task being worked, and the poll on the open MRs.
    pub(super) fn clock(&mut self) {
        let now = groove_types::Timestamp::now();
        groove_controllers::task::time::tick(&mut self.state, &self.services, &self.spawner, now);
        groove_controllers::delivery::poll(&mut self.state, &self.services, &self.spawner, now);
    }
}
