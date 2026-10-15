//! One line of the feed: the kind's mark, what was done and when, over what it was done to.

use groove_controllers::AppState;
use groove_gfx::{Align, Edges, Rect};
use groove_types::{SessionId, TimelineKind, Timestamp};

use super::Line;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{hoverable, square};
use groove_ui_kit::text::{Label, ago};

/// What a line says: when it was, its mark, what was done, and what it was done to.
struct Said<'a> {
    at: Timestamp,
    mark: Mark,
    act: &'a str,
    subject: String,
    role: Role,
}

/// One line of the feed, drawn in the row the list gives it; `every` names each event's session.
pub(super) fn draw(ctx: &mut Ctx, line: Rect, one: &Line<'_>, (app, every): (&AppState, bool)) {
    let said = match one {
        Line::Job(label) => return running(ctx, line, label),
        Line::Bad(bad) => Said {
            at: bad.at,
            mark: Mark::Close,
            act: "error",
            subject: bad.what.message.clone(),
            role: Role::Bad,
        },
        Line::Said(said) => Said {
            at: said.at,
            mark: Mark::Note,
            act: "note",
            subject: said.what.clone(),
            role: Role::Accent,
        },
        Line::Event(event) => {
            reachable(ctx, line, &event.session);
            let title = every.then(|| titled(app, &event.session)).flatten();
            Said {
                at: event.at,
                mark: mark_of(event.kind),
                act: event.kind.label(),
                subject: match title {
                    Some(title) if event.subject.is_empty() => title.to_string(),
                    Some(title) => format!("{title} \u{b7} {}", event.subject),
                    None => event.subject.clone(),
                },
                role: Role::Accent,
            }
        }
    };
    over(ctx, line, said);
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

/// The line takes the pointer to the session it belongs to.
fn reachable(ctx: &mut Ctx, line: Rect, session: &SessionId) {
    let target = Target::FeedLine(session.clone());
    hoverable(ctx, line, target);
}

/// A job the user waits on, with the mark that keeps turning in the mark's place.
fn running(ctx: &mut Ctx, line: Rect, label: &str) {
    let (mut top, _) = halves(ctx, line);
    let size = ctx.tokens.small;
    let slot = top.take_left(ctx.tokens.icon);
    top.take_left(ctx.tokens.sm);
    let box_ = slot.align((size, size), Align::Center, Align::Center);
    groove_ui_kit::widgets::busy(ctx, box_, Role::Working);
    Label::new(label, ctx.styles.strong(Role::Working)).draw(ctx, top);
}

/// The mark, the act and its age on the right, then the subject under the act.
fn over(ctx: &mut Ctx, line: Rect, said: Said<'_>) {
    let (mut top, mut under) = halves(ctx, line);
    let size = ctx.tokens.icon;
    let slot = top.take_left(size);
    top.take_left(ctx.tokens.sm);
    under.take_left(size + ctx.tokens.sm);
    let mark_role = match said.role {
        Role::Bad => Role::Bad,
        _ => Role::Faint,
    };
    groove_ui_kit::widgets::icon(ctx, square(slot, size), said.mark, mark_role);
    let when = ago(said.at.age_at(ctx.now));
    Label::new(&when, ctx.styles.small(Role::Ghost)).right(ctx, &mut top, 0.0);
    top.take_right(ctx.tokens.sm);
    Label::new(said.act, ctx.styles.strong(said.role)).draw(ctx, top);
    if !said.subject.is_empty() {
        Label::new(&said.subject, ctx.styles.small(Role::Muted)).draw(ctx, under);
    }
}

/// The line's two halves, inside its margins.
fn halves(ctx: &Ctx, line: Rect) -> (Rect, Rect) {
    let mut top = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let under = top.take_bottom(line.h / 2.0);
    (top, under)
}
