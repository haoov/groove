//! The Resources tab's own line: the contexts picked, then the namespaces, each a picker.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::widgets::{lead, picker};

use crate::ctx::Ctx;
use crate::hit::{Picks, Target};
use crate::views::session::resources::{self, ResourcesUi};

pub(super) fn draw(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open, held: &ResourcesUi) {
    let contexts = resources::contexts(open);
    let shown: Vec<&str> = contexts
        .iter()
        .copied()
        .filter(|one| !held.hidden_contexts.contains(*one))
        .collect();
    let contexts = named(&shown, contexts.len(), "all contexts");
    let offered: Vec<_> = resources::offered(open, held).collect();
    let pairs: Vec<String> = resources::held(open, held).map(pair).collect();
    let pairs: Vec<&str> = pairs.iter().map(String::as_str).collect();
    let namespaces = named(&pairs, offered.len(), "all namespaces");
    let (band, hover) = (ctx.styles.band(), ctx.styles.hover());
    let role = |empty: bool| if empty { Role::Faint } else { Role::Text };
    let mut room = line.pad(Edges::across(ctx.tokens.md, 0.0));
    let x = lead(ctx, &mut room, Mark::Cluster, Role::Faint).right() + ctx.tokens.sm;
    let hue = match shown.as_slice() {
        [one] => app.config.cluster(one).map(|held| Role::Hue(held.hue)),
        _ => None,
    };
    let first = picker(
        &contexts,
        Target::Picker(Picks::Contexts),
        hue.unwrap_or(role(shown.is_empty())),
        band,
        hover,
    );
    let box_ = first.at(ctx, line, x);
    let second = picker(
        &namespaces,
        Target::Picker(Picks::Namespaces),
        role(pairs.is_empty()),
        band,
        hover,
    );
    second.at(ctx, line, box_.right() + ctx.tokens.sm);
}

/// `paxone`, `*` for the whole cluster, after its context when several are held.
fn pair(one: &groove_types::Attached) -> String {
    one.namespace.as_deref().unwrap_or("*").to_string()
}

/// Every one picked reads as `all`; none as `none`; else the picked ones.
fn named(shown: &[&str], of: usize, all: &str) -> String {
    match shown.len() {
        0 => "none".into(),
        n if n == of && n > 1 => all.into(),
        _ => shown.join(", "),
    }
}
