//! The overrides a rebinding leaves: one action's new chord, taken off whatever held it.

use std::collections::BTreeMap;

use groove_types::{Chord, Config};

use super::{Action, Keymap, TABLE};

type Overrides = BTreeMap<String, Vec<String>>;

/// `action` on `chord` alone; any other action holding the chord gives it up.
pub fn rebound(config: Option<&Config>, action: Action, chord: Chord) -> Overrides {
    let keymap = Keymap::of(config);
    let mut out = held(config);
    for spec in TABLE.iter().filter(|spec| spec.action != action) {
        let chords = keymap.chords(spec.action);
        if chords.contains(&chord) {
            let kept: Vec<Chord> = chords.iter().copied().filter(|one| *one != chord).collect();
            set(&mut out, spec.action, &kept);
        }
    }
    set(&mut out, action, &[chord]);
    out
}

/// `action` back on its defaults.
pub fn reset(config: Option<&Config>, action: Action) -> Overrides {
    let mut out = held(config);
    if let Some(spec) = TABLE.iter().find(|spec| spec.action == action) {
        out.remove(spec.id);
    }
    out
}

/// Whether the config binds `action` to something other than its defaults.
pub fn rebinds(config: Option<&Config>, action: Action) -> bool {
    let id = TABLE
        .iter()
        .find(|spec| spec.action == action)
        .map(|spec| spec.id);
    id.is_some_and(|id| held(config).contains_key(id))
}

fn held(config: Option<&Config>) -> Overrides {
    config.map(|c| c.keymap.clone()).unwrap_or_default()
}

/// One action's chords written, or its override dropped when they are its defaults.
fn set(out: &mut Overrides, action: Action, chords: &[Chord]) {
    let Some(spec) = TABLE.iter().find(|spec| spec.action == action) else {
        return;
    };
    let texts: Vec<String> = chords.iter().map(ToString::to_string).collect();
    let defaults: Vec<String> = spec
        .defaults
        .iter()
        .filter_map(|one| Chord::parse(one))
        .map(|one| one.to_string())
        .collect();
    match texts == defaults {
        true => out.remove(spec.id),
        false => out.insert(spec.id.to_string(), texts),
    };
}
