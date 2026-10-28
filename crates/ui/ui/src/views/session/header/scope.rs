//! The scope line: the contexts shown, then the namespaces; `alt+c` and `alt+n` change them.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::widgets::{Button, Text, lead};

use crate::ctx::Ctx;
use crate::hit::{Picks, Target};
use crate::views::session::resources::{self, ResourcesUi};

pub(super) fn draw(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open, held: &ResourcesUi) {
    let contexts = resources::contexts(open);
    let shown = resources::shown_contexts(open, held);
    let none = contexts.is_empty();
    let contexts = match none {
        true => "no cluster".to_string(),
        false => named(&shown, contexts.len(), "all contexts"),
    };
    let offered: Vec<_> = resources::offered(open, held).collect();
    let pairs: Vec<String> = resources::held(open, held).iter().map(pair).collect();
    let pairs: Vec<&str> = pairs.iter().map(String::as_str).collect();
    let namespaces = named(&pairs, offered.len(), "all namespaces");
    let band = ctx.styles.band();
    let shown_as = |label, which, role| {
        Button::new(label, Target::Picker(which), role, band)
            .text(Text::Body)
            .flat()
            .inert(true)
    };
    let role = |empty: bool| if empty { Role::Faint } else { Role::Text };
    let mut room = line.pad(Edges::across(ctx.tokens.md, 0.0));
    let x = lead(ctx, &mut room, Mark::Cluster, Role::Faint).right() + ctx.tokens.sm;
    let hue = match shown.as_slice() {
        [one] => app.config.cluster(one).map(|held| Role::Hue(held.hue)),
        _ => None,
    };
    let role_of = hue.unwrap_or(role(shown.is_empty()));
    let first = shown_as(&contexts, Picks::Contexts, role_of);
    let box_ = first.at(ctx, line, x);
    if none {
        return;
    }
    let second = shown_as(&namespaces, Picks::Namespaces, role(pairs.is_empty()));
    second.at(ctx, line, box_.right() + ctx.tokens.sm);
}

/// `shop`, `*` for the whole cluster, after its context when several are held.
fn pair(one: &groove_types::Attached) -> String {
    let whole = resources::WHOLE_MARK;
    one.namespace.as_deref().unwrap_or(whole).to_string()
}

/// Every one picked reads as `all`; none as `none`; else the picked ones.
fn named(shown: &[&str], of: usize, all: &str) -> String {
    match shown.len() {
        0 => "none".into(),
        n if n == of && n > 1 => all.into(),
        _ => shown.join(", "),
    }
}
