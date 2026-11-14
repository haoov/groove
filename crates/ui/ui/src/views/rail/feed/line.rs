//! One line of the feed's log: how long ago, a short code for its kind, then what happened.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{SessionId, TimelineKind, Timestamp};
use groove_ui_kit::shape::from_left;

use super::super::RunKey;
use super::Line;
use super::runs::{Item, Run};
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::{Label, ago};
use groove_ui_kit::widgets::{flat_ground, flat_row};

/// What a line writes in its three columns, and the colour of its code and its text.
struct Logged {
    age: String,
    code: &'static str,
    text: String,
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
        Line::Job(label) => {
            let job = Logged {
                age: "··".into(),
                code: "job",
                text: label.to_string(),
                role: Role::Working,
            };
            logged(ctx, line, &job);
        }
        Line::Run(run) => folded(ctx, line, run, (app, every, open)),
        Line::Under(item, last) => {
            reachable(ctx, line, *item);
            let mut said = said(app, *item, (false, ctx.now));
            said.code = if *last { "└" } else { "│" };
            logged(ctx, line, &said);
        }
    }
}

/// A run's line: one event as it is, or several as one with their count, open or shut.
fn folded(
    ctx: &mut Ctx,
    line: Rect,
    run: &Run<'_>,
    (app, every, open): (&AppState, bool, Option<RunKey>),
) {
    let newest = run.items[0];
    if let Item::Bad(_) = newest {
        flat_ground(ctx, line, ctx.styles.row_tint(Role::Bad));
    }
    let mut said = said(app, newest, (every, ctx.now));
    let count = run.items.len();
    if count > 1 {
        let key = run.key();
        let shown = open == Some(key);
        flat_row(ctx, line, Target::FeedRun(key), shown);
        let turn = if shown { '▾' } else { '▸' };
        said.text = format!("{} ×{count} {turn}", act(newest));
    } else {
        reachable(ctx, line, newest);
    }
    logged(ctx, line, &said);
}

/// The three columns: the age and the code each as wide as their longest, the text in the rest.
fn logged(ctx: &mut Ctx, line: Rect, said: &Logged) {
    let (sm, md) = (ctx.tokens.sm, ctx.tokens.md);
    let mut room = line.pad(Edges::across(md, md));
    let ghost = ctx.styles.log(Role::Ghost);
    let age_wide = ctx.measure("00m", &ghost);
    let age = from_left(&mut room, age_wide, sm);
    Label::new(&said.age, ghost).draw(ctx, age);
    let code_role = match said.role {
        Role::Text => Role::Faint,
        other => other,
    };
    let code_style = ctx.styles.log(code_role);
    let code_wide = ctx.measure("note", &code_style);
    let code = from_left(&mut room, code_wide, sm);
    Label::new(said.code, code_style).draw(ctx, code);
    Label::new(&said.text, ctx.styles.log(said.role)).draw(ctx, room);
}

/// What one item writes, its session named when `every`, its age at `now`.
fn said(app: &AppState, item: Item<'_>, (every, now): (bool, Timestamp)) -> Logged {
    let age = ago(item.at().age_at(now));
    match item {
        Item::Bad(bad) => Logged {
            age,
            code: "err",
            text: bad.what.message.clone(),
            role: Role::Bad,
        },
        Item::Said(said) => Logged {
            age,
            code: "note",
            text: said.what.clone(),
            role: Role::Text,
        },
        Item::Event(event) => {
            let title = every.then(|| titled(app, &event.session)).flatten();
            let what = match event.subject.is_empty() {
                true => act(item).to_string(),
                false => format!("{} {}", act(item), event.subject),
            };
            Logged {
                age,
                code: code_of(event.kind),
                text: match title {
                    Some(title) => format!("{title} \u{b7} {what}"),
                    None => what,
                },
                role: Role::Text,
            }
        }
    }
}

/// What an item did, as a word or two.
fn act(item: Item<'_>) -> &'static str {
    match item {
        Item::Bad(_) => "error",
        Item::Said(_) => "note",
        Item::Event(event) => match event.kind {
            TimelineKind::TurnStart | TimelineKind::TurnEnd => "turn",
            kind => kind.label(),
        },
    }
}

/// The short code of a kind of event, four letters at most.
fn code_of(kind: TimelineKind) -> &'static str {
    match kind {
        TimelineKind::TurnStart | TimelineKind::TurnEnd => "agt",
        TimelineKind::Commit
        | TimelineKind::Push
        | TimelineKind::Pull
        | TimelineKind::Rebase
        | TimelineKind::RepoAdded
        | TimelineKind::WorktreeAdded
        | TimelineKind::WorktreeRemoved => "git",
        TimelineKind::MrOpened
        | TimelineKind::MrUpdated
        | TimelineKind::MrMerged
        | TimelineKind::MrClosed => "mr",
        TimelineKind::Review | TimelineKind::Note => "note",
        TimelineKind::RoutineRan | TimelineKind::RoutineEnded => "rtn",
        TimelineKind::ClusterRead => "kube",
    }
}

/// The session's title, open or only on disk.
fn titled<'a>(app: &'a AppState, session: &SessionId) -> Option<&'a str> {
    let open = app.session.open.iter().map(|one| &one.session);
    let living = app.session.living.iter().map(|one| &one.session);
    let found = open.chain(living).find(|one| &one.id == session)?;
    Some(found.title.as_str())
}

/// The line takes the pointer to the session it belongs to, when it has one.
fn reachable(ctx: &mut Ctx, line: Rect, item: Item<'_>) {
    if let Some(session) = item.session() {
        flat_row(ctx, line, Target::FeedLine(session.clone()), false);
    }
}
