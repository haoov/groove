//! The Kubernetes sidebar: the lists pinned, the objects open, the kinds.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::Pin;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, Heading, cap, glass_chosen, glass_hoverable};

use super::super::opened::Opened;
use super::super::sidebar::{self, kind_line};
use super::super::{list, pins, scope};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};

/// One row of the sidebar, as drawn and as the arrows walk it.
pub(crate) enum Line<'a> {
    /// `LISTS · 2`, `OPEN · 1`, `KINDS`.
    Head(&'static str, usize),
    Pinned(&'a Pin),
    Open(usize, &'a Opened),
    Kind(sidebar::Line),
}

impl Line<'_> {
    /// What a click on the row does, if anything.
    pub(crate) fn target(&self) -> Option<Target> {
        match self {
            Line::Head(..) => None,
            Line::Pinned(pin) => Some(Target::PinnedList(Box::new((*pin).clone()))),
            Line::Open(at, _) => Some(Target::ResourceTab(*at)),
            Line::Kind(sidebar::Line::Heading(group, _)) => {
                Some(Target::ResourceGroup(group.clone()))
            }
            Line::Kind(sidebar::Line::Kind(kind)) => Some(Target::ResourceKind(kind.clone())),
        }
    }
}

/// `KUBERNETES · 2 contexts`.
pub fn head(ctx: &mut Ctx, app: &AppState, row: Rect, shown: bool) {
    let count = app
        .session
        .selected()
        .map_or(0, |one| scope::contexts(one).len());
    let said = match count {
        1 => "1 context".to_string(),
        many => format!("{many} contexts"),
    };
    Heading::new("kubernetes")
        .note(&said)
        .fold(shown)
        .draw(ctx, row);
}

/// The pinned lists, the objects open, then the kinds under their headings.
pub(crate) fn lines<'a>(app: &'a AppState, ui: &'a Ui) -> Vec<Line<'a>> {
    let mut out = Vec::new();
    let pinned = app.session.selected().map(|one| one.pins.as_slice());
    let pinned = pinned.unwrap_or_default();
    if !pinned.is_empty() {
        out.push(Line::Head("lists", pinned.len()));
        out.extend(pinned.iter().map(Line::Pinned));
    }
    let opened = &ui.session.resources.opened;
    if !opened.is_empty() {
        out.push(Line::Head("open", opened.len()));
        out.extend(
            opened
                .iter()
                .enumerate()
                .map(|(at, one)| Line::Open(at, one)),
        );
    }
    out.push(Line::Head("kinds", 0));
    out.extend(sidebar::shown(app, ui).into_iter().map(Line::Kind));
    out
}

/// The rows in `room`, scrolled; returns how far they scroll.
pub fn body(ctx: &mut Ctx, app: &AppState, ui: &Ui, room: Rect) -> f32 {
    let lines = lines(app, ui);
    let held = &ui.session.resources;
    let open = app.session.selected();
    let kind = open.and_then(|one| scope::kind(app, one, held));
    let shown = open.and_then(|one| pins::shown(app, one, held));
    let at = (Scroller::Kinds, held.kinds);
    let height = ctx.tokens.row;
    crate::offsets::listed(
        ctx,
        room,
        at,
        &lines,
        |_| height,
        |ctx, line, one| match one {
            Line::Head(name, count) => {
                let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
                let said = match count {
                    0 => name.to_string(),
                    n => format!("{name} · {n}"),
                };
                cap(ctx, &mut room, &said, Role::Ghost);
            }
            Line::Pinned(pin) => pinned(ctx, line, pin, shown.as_ref() == Some(*pin)),
            Line::Open(at, one) => opened(ctx, line, (*at, one), held.showing == Some(*at)),
            Line::Kind(one) => kind_line(ctx, line, one, kind.as_ref()),
        },
    )
}

/// The cross at the right of `room` while the row or the cross is under the pointer.
fn closing(ctx: &mut Ctx, room: &mut Rect, on: bool, close: Target) {
    if on || ctx.hovered(&close) {
        let cross = Button::icon(Mark::Close, 0, close, Role::Faint);
        cross.right(ctx, room, ctx.tokens.sm);
    }
}

/// `Pods  staging / shop`: held while it is the list shown.
fn pinned(ctx: &mut Ctx, line: Rect, pin: &Pin, held: bool) {
    if held {
        glass_chosen(ctx, line);
    }
    let on = glass_hoverable(ctx, line, Target::PinnedList(Box::new(pin.clone())));
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.sm));
    closing(ctx, &mut room, on, Target::Unpin(Box::new(pin.clone())));
    let sm = ctx.tokens.sm;
    let kind = list::plural(&pin.kind);
    Label::new(&kind, ctx.styles.body(Role::Text)).left(ctx, &mut room, sm);
    let scope = pins::scope_of(pin);
    Label::new(&scope, ctx.styles.small(Role::Ghost)).draw(ctx, room);
}

/// `POD  api-0`: held while it shows.
fn opened(ctx: &mut Ctx, line: Rect, (at, one): (usize, &Opened), held: bool) {
    if held {
        glass_chosen(ctx, line);
    }
    let on = glass_hoverable(ctx, line, Target::ResourceTab(at));
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.sm));
    closing(ctx, &mut room, on, Target::ResourceClose(at));
    cap(ctx, &mut room, &one.link.kind.kind, Role::Ghost);
    let name = Label::new(&one.link.name, ctx.styles.body(Role::Text));
    name.draw(ctx, room);
}
