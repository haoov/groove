//! The choices of *Pick a kind*: every kind the session's contexts serve, and the one a choice names.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_types::KubeKind;

/// What separates a kind's group from its name in a choice's value.
const BETWEEN: char = '\u{1}';

/// `Deployment (deploy) · apps`: the kind, its short names, its group.
fn label(kind: &KubeKind) -> String {
    let mut out = kind.kind.clone();
    if !kind.short_names.is_empty() {
        out.push_str(&format!(" ({})", kind.short_names.join(", ")));
    }
    if !kind.group.is_empty() {
        out.push_str(&format!(" \u{b7} {}", kind.group));
    }
    out
}

/// Every kind the contexts held serve, by name.
fn served<'a>(app: &'a AppState, open: &Open) -> Vec<&'a KubeKind> {
    let contexts = crate::views::session::resources::contexts(open);
    let mut kinds = crate::views::session::resources::served(app, contexts);
    kinds.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.group.cmp(&b.group)));
    kinds
}

pub(super) fn choices(app: &AppState, open: &Open) -> Vec<(String, String)> {
    let value = |kind: &KubeKind| format!("{}{BETWEEN}{}", kind.group, kind.kind);
    let kinds = served(app, open).into_iter();
    kinds.map(|one| (label(one), value(one))).collect()
}

/// The kind a choice's value names.
pub fn chosen(app: &AppState, open: &Open, value: &str) -> Option<KubeKind> {
    let (group, kind) = value.split_once(BETWEEN)?;
    let found = served(app, open).into_iter();
    found
        .into_iter()
        .find(|one| one.group == group && one.kind == kind)
        .cloned()
}
