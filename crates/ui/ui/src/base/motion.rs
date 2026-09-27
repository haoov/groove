//! What moves with the clock.

use groove_gfx::Icon;

use crate::base::tokens::SPINNER_MS;

/// Which eighth of a turn a busy mark is at.
pub fn turn(tick: u64) -> u8 {
    (tick / SPINNER_MS % u64::from(Icon::TURNS)) as u8
}
