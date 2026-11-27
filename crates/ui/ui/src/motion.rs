//! What moves when the user opens something over the window: a palette, a menu, a scope panel
//! appearing. Nothing moves by itself; with reduced motion, nothing moves.

use groove_controllers::AppState;

use crate::{Overlay, Ui};

/// How long a panel over the window takes to appear, and a frame while it does, in ms.
pub const OPEN_MS: u64 = 140;
pub const MOTION_MS: u64 = 16;

/// What the last frame showed, and what is moving away from it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Motion {
    over: Option<Over>,
    /// When what stands over the window opened.
    opened: Option<u64>,
}

/// What opens over the window and appears as it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Over {
    Palette,
    Menu,
    Scope,
    Prompt,
}

impl Over {
    fn of(overlay: &Overlay) -> Option<Self> {
        match overlay {
            Overlay::Palette(_) => Some(Over::Palette),
            Overlay::Menu(_) => Some(Over::Menu),
            Overlay::Scope(_) => Some(Over::Scope),
            Overlay::Prompt => Some(Over::Prompt),
            Overlay::Losing(_) | Overlay::Examining(_) => None,
        }
    }
}

impl Motion {
    /// How far what stands over the window has appeared, eased: whole once it has.
    pub fn opening(&self, tick: u64) -> f32 {
        let since = |at: u64| eased(tick.saturating_sub(at), OPEN_MS);
        self.opened.and_then(since).unwrap_or(1.0)
    }

    /// Whether anything still moves, and frames must come.
    pub fn moving(&self, tick: u64) -> bool {
        self.opening(tick) < 1.0
    }

    fn note(&mut self, over: Option<Over>, tick: u64, still: bool) {
        if over != self.over {
            self.opened = (over.is_some() && !still).then_some(tick);
        }
        self.over = over;
    }
}

impl Ui {
    /// Notes what this frame shows: a panel opened over the window starts appearing.
    pub fn animate(&mut self, app: &AppState, tick: u64) {
        let over = self.overlay.as_ref().and_then(Over::of);
        let still = app.config.reduce_motion();
        self.motion.note(over, tick, still);
    }
}

/// `since` of `length` ms, eased out; none once it has run.
fn eased(since: u64, length: u64) -> Option<f32> {
    (since < length).then(|| ease(since as f32 / length as f32))
}

/// `t` eased out: quick, then settling.
pub fn ease(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}
