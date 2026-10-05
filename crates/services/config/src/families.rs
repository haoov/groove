//! The vendored family mono text draws in.

use groove_types::FontFamily;

use crate::State;

impl State {
    /// Code, the agent and the terminals.
    pub fn mono_family(&self) -> FontFamily {
        self.config
            .as_ref()
            .map(|c| FontFamily::named(&c.ui.mono_font_family))
            .unwrap_or_default()
    }
}
