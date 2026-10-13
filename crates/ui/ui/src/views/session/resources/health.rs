//! The colour a health stands in.

use groove_types::Health;
use groove_ui_kit::base::style::Role;

pub(crate) fn colour(health: Health) -> Role {
    match health {
        Health::Good => Role::Ok,
        Health::Waiting => Role::Warn,
        Health::Failing => Role::Bad,
        Health::Done => Role::Ghost,
    }
}

/// A status word's colour; a word it does not know waits.
pub(crate) fn status_colour(word: &str) -> Role {
    colour(groove_types::status_health(word).unwrap_or(Health::Waiting))
}
