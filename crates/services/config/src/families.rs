//! The vendored families the interface and mono text draw in.

use groove_types::{FontFamily, UiConfig};

use crate::State;

impl State {
    pub fn ui_family(&self) -> FontFamily {
        self.family(|ui| &ui.font_family, &FontFamily::UI)
    }

    /// Code, the agent and the terminals.
    pub fn mono_family(&self) -> FontFamily {
        self.family(|ui| &ui.mono_font_family, &FontFamily::MONO)
    }

    fn family(&self, of: impl Fn(&UiConfig) -> &String, among: &[FontFamily]) -> FontFamily {
        self.config
            .as_ref()
            .map(|c| FontFamily::named(of(&c.ui), among))
            .unwrap_or_default()
    }
}
