//! One row of the conversation: how tall it stands at a width, and how it draws.

use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::chat::{Edit, Entry, Tool};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::SPEAKER_SHARE;
use groove_ui_kit::markdown::{self, prose};
use groove_ui_kit::shape::{band, from_left, from_right, strip};
use groove_ui_kit::text::{Label, row, rows, wrapped};
use groove_ui_kit::widgets::{changes, changes_room, glass_hoverable, glass_tinted, lead};

use crate::ctx::Ctx;
use crate::hit::Target;

/// A row the conversation draws: an entry, or a message waiting for the agent's turn.
#[derive(Clone, Copy)]
pub(super) enum Row<'a> {
    Entry(&'a Entry),
    Queued(&'a str),
}

pub(super) fn height(ctx: &mut Ctx, shown: Row, (width, opened): (f32, &[String])) -> f32 {
    let width = width * SPEAKER_SHARE;
    let entry = match shown {
        Row::Entry(entry) => entry,
        Row::Queued(text) => return bubble_height(ctx, text, width),
    };
    match entry {
        Entry::You(text) => bubble_height(ctx, text, width),
        Entry::Said(text) => markdown::height(ctx, text, width, Role::Text),
        Entry::Glance(glance) => {
            let opened = super::glance::opened(glance, opened);
            super::glance::height(ctx, glance, opened, width)
        }
        Entry::Edit(_) | Entry::Tool(_) | Entry::Interrupted => ctx.tokens.row,
    }
}

/// One row; `live` while it is the last and the agent works.
pub(super) fn draw(
    ctx: &mut Ctx,
    (open, live, opened): (&Open, bool, &[String]),
    shown: Row,
    (whole, visible): (Rect, Rect),
) {
    let at = strip(whole, 0.0, whole.w * SPEAKER_SHARE);
    let entry = match shown {
        Row::Queued(text) => return you(ctx, whole, text, Role::Faint),
        Row::Entry(Entry::You(text)) => return you(ctx, whole, text, Role::Text),
        Row::Entry(entry) => entry,
    };
    match entry {
        Entry::You(_) => {}
        Entry::Said(text) => {
            let link = |url: &str| Target::Link(url.to_string());
            prose(ctx, (at, visible), text, Role::Text, link);
        }
        Entry::Glance(one) => {
            let opened = super::glance::opened(one, opened);
            super::glance::draw(ctx, at, one, (live, opened))
        }
        Entry::Edit(edit) => edited(ctx, open, at, edit),
        Entry::Tool(tool) => called(ctx, at, tool),
        Entry::Interrupted => {
            let mut room = at;
            lead(ctx, &mut room, Mark::Close, Role::Warn);
            Label::new("interrupted", ctx.styles.body(Role::Faint)).draw(ctx, room);
        }
    }
}

fn bubble_height(ctx: &mut Ctx, text: &str, width: f32) -> f32 {
    let style = ctx.styles.body(Role::Text);
    let inner = width - bubble_pad(ctx) * 2.0;
    rows(ctx, text, &style, inner) as f32 * ctx.tokens.line + ctx.tokens.sm * 2.0
}

/// The room a bubble leaves between its rim and its words.
fn bubble_pad(ctx: &Ctx) -> f32 {
    ctx.tokens.lg + ctx.tokens.xs
}

/// The user's words on a rounded ground at the right, as wide as the agent's side.
fn you(ctx: &mut Ctx, at: Rect, text: &str, role: Role) {
    let (sm, pad, line) = (ctx.tokens.sm, bubble_pad(ctx), ctx.tokens.line);
    let style = ctx.styles.body(role);
    let most = at.w * SPEAKER_SHARE;
    let lines = wrapped(ctx, text, &style, most - pad * 2.0);
    let widest = lines
        .iter()
        .map(|one| ctx.measure(one, &style))
        .fold(0.0, f32::max);
    let mut room = at;
    let bubble = from_right(&mut room, (widest + pad * 2.0).min(most), 0.0);
    glass_tinted(ctx, bubble, ctx.styles.glass_chosen());
    let inside = bubble.pad(Edges::xy(pad, sm));
    for (n, one) in lines.iter().enumerate() {
        row(ctx, band(inside, n as f32 * line, line), 0.0, one, style);
    }
}

pub(super) fn counted(count: usize, one: &str) -> String {
    match count {
        1 => format!("1 {one}"),
        many => format!("{many} {one}s"),
    }
}

/// A file written: the verb, and a link to its hunk with what it gained and lost.
fn edited(ctx: &mut Ctx, open: &Open, at: Rect, edit: &Edit) {
    let mut room = at;
    let (verb, mark, role) = match edit.created {
        true => ("write", Mark::File, Role::Ok),
        false => ("edit", Mark::Edited, Role::Warn),
    };
    lead(ctx, &mut room, mark, role);
    Label::new(verb, ctx.styles.body(Role::Faint)).left(ctx, &mut room, ctx.tokens.sm);
    let shown = super::within(open, &edit.path).map_or_else(|| tail(&edit.path), |(_, rel)| rel);
    let style = ctx.styles.code(Role::Text);
    let counts = edit.patch.map(|one| (one.added, one.removed));
    let counted = counts.map_or(0.0, |counts| {
        changes_room(ctx, counts, |ctx, role| ctx.styles.code(role))
    });
    let wide = ctx.measure(&shown, &style) + ctx.tokens.md * 2.0 + counted;
    let wide = wide.min(room.w);
    let chip = from_left(&mut room, wide, ctx.tokens.sm);
    let line = edit.patch.map_or(1, |one| one.line);
    let path = edit.path.clone();
    glass_hoverable(ctx, chip, Target::Edited { path, line });
    let mut inside = chip.pad(Edges::across(ctx.tokens.md, 0.0));
    if let Some(counts) = counts {
        changes(ctx, &mut inside, counts, |ctx, role| ctx.styles.code(role));
    }
    Label::new(&shown, style).cut_start().draw(ctx, inside);
    if edit.failed {
        Label::new("failed", ctx.styles.small(Role::Bad)).draw(ctx, room);
    }
}

/// A path outside the session's worktrees, by its last two parts.
fn tail(path: &str) -> String {
    let parts: Vec<&str> = path.rsplit('/').take(2).collect();
    parts.into_iter().rev().collect::<Vec<_>>().join("/")
}

/// Another tool: its name, what it was given, and how it ended.
fn called(ctx: &mut Ctx, at: Rect, tool: &Tool) {
    let mut room = at;
    lead(ctx, &mut room, Mark::Outward, Role::Faint);
    let (word, role) = match tool.failed {
        None => ("running", Role::Working),
        Some(true) => ("failed", Role::Bad),
        Some(false) => ("done", Role::Ghost),
    };
    Label::new(word, ctx.styles.small(role)).right(ctx, &mut room, ctx.tokens.sm);
    Label::new(&tool.name, ctx.styles.body(Role::Muted)).left(ctx, &mut room, ctx.tokens.sm);
    if let Some(detail) = &tool.detail {
        Label::new(detail, ctx.styles.code(Role::Text)).draw(ctx, room);
    }
}
