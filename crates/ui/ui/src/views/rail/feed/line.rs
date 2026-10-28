//! One line of the feed: the kind's mark, what was done and when, over what it was done to.

use groove_controllers::AppState;
use groove_gfx::{Align, Edges, Rect};
use groove_types::{SessionId, TimelineKind, Timestamp};
use groove_ui_kit::shape::{from_bottom, from_left};

use super::super::RunKey;
use super::Line;
use super::runs::Item;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::circle;
use groove_ui_kit::text::{Label, ago};
use groove_ui_kit::widgets::{Badge, card_inset, glass_hoverable, glass_tinted, rounded_slice};

/// What a line says: when it was, its mark, what was done, and what it was done to.
struct Said<'a> {
    at: Timestamp,
    mark: Mark,
    act: &'a str,
    subject: String,
    role: Role,
}

/// One line of the feed, drawn in the row the list gives it; `every` names each event's session.
pub(super) fn draw(
    ctx: &mut Ctx,
    line: Rect,
    one: &Line<'_>,
    (app, every, open): (&AppState, bool, Option<RunKey>),
) {
    match one {
        Line::Job(label) => running(ctx, line, label),
        Line::Run(run) => {
            let newest = run.items[0];
            let count = run.items.len();
            if let Item::Bad(_) = newest {
                glass_tinted(ctx, line, ctx.styles.row_tint(Role::Bad));
            }
            match count {
                1 => reachable(ctx, line, newest),
                _ => {
                    let key = run.key();
                    if open == Some(key) {
                        held(ctx, line, (true, false));
                    }
                    glass_hoverable(ctx, line, Target::FeedRun(key));
                }
            }
            let mut said = said(app, newest, every);
            if count > 1 {
                said.subject.clear();
            }
            over(ctx, line, said, count);
        }
        Line::Under(item, last) => {
            held(ctx, line, (false, *last));
            let lead = ctx.tokens.md + lead(ctx);
            reachable(ctx, line.pad(Edges::across(lead, 0.0)), *item);
            under(ctx, line, item.at(), &said(app, *item, false).subject);
        }
    }
}

/// An open run's slice of its held ground, rounded where the run opens or closes.
fn held(ctx: &mut Ctx, line: Rect, (top, bottom): (bool, bool)) {
    let (sm, half) = (ctx.tokens.sm, ctx.tokens.xs / 2.0);
    let slice = line.pad(Edges {
        top: if top { half } else { 0.0 },
        bottom: if bottom { half } else { 0.0 },
        left: sm,
        right: sm,
    });
    let ground = ctx.styles.glass_chosen();
    rounded_slice(ctx, slice, (top, bottom), (ground, None));
}

/// The room the mark's round takes, before the act and under it.
fn lead(ctx: &Ctx) -> f32 {
    ctx.tokens.icon + ctx.tokens.sm
}

/// The kind's mark in a round in `role`'s tint, or see-through, at the left of `top`.
fn marked(ctx: &mut Ctx, top: &mut Rect, role: Option<Role>) -> Rect {
    let slot = from_left(top, lead(ctx), ctx.tokens.sm);
    let ground = match role {
        Some(role) => ctx.styles.tint(role),
        None => ctx.styles.glass_hover(),
    };
    circle(ctx, slot, ground);
    let size = ctx.tokens.small;
    slot.align((size, size), Align::Center, Align::Center)
}

/// What one item says, its session named when `every`.
fn said<'a>(app: &AppState, item: Item<'a>, every: bool) -> Said<'a> {
    match item {
        Item::Bad(bad) => Said {
            at: bad.at,
            mark: Mark::Close,
            act: "error",
            subject: bad.what.message.clone(),
            role: Role::Bad,
        },
        Item::Said(said) => Said {
            at: said.at,
            mark: Mark::Note,
            act: "note",
            subject: said.what.clone(),
            role: Role::Text,
        },
        Item::Event(event) => {
            let title = every.then(|| titled(app, &event.session)).flatten();
            Said {
                at: event.at,
                mark: mark_of(event.kind),
                act: match event.kind {
                    TimelineKind::TurnStart | TimelineKind::TurnEnd => "turn",
                    kind => kind.label(),
                },
                subject: match title {
                    Some(title) if event.subject.is_empty() => title.to_string(),
                    Some(title) => format!("{title} \u{b7} {}", event.subject),
                    None => event.subject.clone(),
                },
                role: Role::Text,
            }
        }
    }
}

/// The session's title, open or only on disk.
fn titled<'a>(app: &'a AppState, session: &SessionId) -> Option<&'a str> {
    let open = app.session.open.iter().map(|one| &one.session);
    let living = app.session.living.iter().map(|one| &one.session);
    let found = open.chain(living).find(|one| &one.id == session)?;
    Some(found.title.as_str())
}

fn mark_of(kind: TimelineKind) -> Mark {
    match kind {
        TimelineKind::TurnStart | TimelineKind::TurnEnd => Mark::Agent,
        TimelineKind::Commit => Mark::Commit,
        TimelineKind::Push => Mark::Ahead,
        TimelineKind::Pull | TimelineKind::Rebase => Mark::Behind,
        TimelineKind::MrOpened
        | TimelineKind::MrUpdated
        | TimelineKind::MrMerged
        | TimelineKind::MrClosed => Mark::Review,
        TimelineKind::Review | TimelineKind::Note => Mark::Note,
        TimelineKind::RepoAdded => Mark::Repo,
        TimelineKind::WorktreeAdded => Mark::Git,
        TimelineKind::WorktreeRemoved => Mark::Close,
        TimelineKind::RoutineRan | TimelineKind::RoutineEnded => Mark::Routine,
        TimelineKind::ClusterRead => Mark::Cluster,
    }
}

/// The line takes the pointer to the session it belongs to, when it has one.
fn reachable(ctx: &mut Ctx, line: Rect, item: Item<'_>) {
    if let Some(session) = item.session() {
        glass_hoverable(ctx, line, Target::FeedLine(session.clone()));
    }
}

/// A job the user waits on, with the mark that keeps turning in the mark's place.
fn running(ctx: &mut Ctx, line: Rect, label: &str) {
    let (mut top, _) = halves(ctx, line);
    let box_ = marked(ctx, &mut top, Some(Role::Working));
    groove_ui_kit::widgets::busy(ctx, box_, Role::Working);
    Label::new(label, ctx.styles.strong(Role::Working)).draw(ctx, top);
}

/// The mark, the act with its count and its age on the right, then the subject under the act.
fn over(ctx: &mut Ctx, line: Rect, said: Said<'_>, count: usize) {
    let (mut top, mut under) = halves(ctx, line);
    let (mark_role, round) = match said.role {
        Role::Bad => (Role::Bad, Some(Role::Bad)),
        _ => (Role::Faint, None),
    };
    let box_ = marked(ctx, &mut top, round);
    from_left(&mut under, lead(ctx), ctx.tokens.sm);
    groove_ui_kit::widgets::icon(ctx, box_, said.mark, mark_role);
    let when = ago(said.at.age_at(ctx.now));
    Label::new(&when, ctx.styles.small(Role::Ghost)).right(ctx, &mut top, ctx.tokens.sm);
    Label::new(said.act, ctx.styles.strong(said.role)).left(ctx, &mut top, ctx.tokens.xs);
    if count > 1 {
        Badge::new(&count.to_string(), Role::Muted).left(ctx, &mut top, 0.0);
    }
    if !said.subject.is_empty() {
        Label::new(&said.subject, ctx.styles.small(Role::Muted)).draw(ctx, under);
    }
}

/// One event of an open run: its subject under the act's column, its age on the right.
fn under(ctx: &mut Ctx, line: Rect, at: Timestamp, subject: &str) {
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    from_left(&mut room, lead(ctx), ctx.tokens.sm);
    let when = ago(at.age_at(ctx.now));
    Label::new(&when, ctx.styles.small(Role::Ghost)).right(ctx, &mut room, ctx.tokens.sm);
    Label::new(subject, ctx.styles.small(Role::Muted)).draw(ctx, room);
}

/// The line's two halves, inside its margins.
fn halves(ctx: &Ctx, line: Rect) -> (Rect, Rect) {
    let (md, inset) = (ctx.tokens.md, card_inset(&ctx.tokens));
    let mut top = line.pad(Edges::xy(md, inset));
    let half = top.h / 2.0;
    let under = from_bottom(&mut top, half, 0.0);
    (top, under)
}
