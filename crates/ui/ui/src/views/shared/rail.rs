use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;
use groove_types::{AgentStatus, AttentionClass, SessionId};

use super::status;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{Row, after_mark, hairline, list};

/// Opened sessions only, in the order opened. The Board row above, the feed below.
pub fn draw(ctx: &mut Ctx, app: &AppState) {
    let rect = ctx.layout.rail;
    let (panel, line, hairline_width) =
        (ctx.styles.panel(), ctx.styles.line(), ctx.tokens.hairline);
    ctx.quad(rect, panel);
    ctx.quad(
        Rect::new(
            rect.right() - hairline_width,
            rect.y,
            hairline_width,
            rect.h,
        ),
        line,
    );

    let width = rect.w - hairline_width;
    let (pad, row_height, gap) = (ctx.tokens.md, ctx.tokens.row, ctx.tokens.sm);
    let board = Row::new(pad, "Board", ctx.styles.label(Role::Text)).mark(Mark::Board);
    let mut y = list(
        ctx,
        Rect::new(0.0, rect.y + gap, width, row_height),
        &[board],
        None,
    ) + gap;

    let sessions: Vec<Entry> = app
        .session
        .open
        .iter()
        .map(|open| {
            let (state, role) = status_of(app, open);
            Entry {
                session: open.session.id.clone(),
                title: open.session.title.clone(),
                mark: Mark::of_kind(&open.session.kind),
                state,
                role,
            }
        })
        .collect();
    let title = ctx.styles.label(Role::Text);
    let second_line = after_mark(ctx, pad);
    let mut rows = Vec::with_capacity(sessions.len() * 2);
    for entry in &sessions {
        let target = Target::Session(entry.session.clone());
        rows.push(
            Row::new(pad, &entry.title, title)
                .mark(entry.mark)
                .target(target.clone()),
        );
        rows.push(Row::new(second_line, &entry.state, ctx.styles.small(entry.role)).target(target));
    }
    y = list(ctx, Rect::new(0.0, y, width, rect.h - y), &rows, None);
    let _ = y;

    let footer = Rect::new(0.0, rect.h - row_height, width, row_height);
    status::draw(ctx, app, footer.y);
    hairline(
        ctx,
        Rect::new(0.0, footer.y - row_height, width, row_height),
        line,
    );
    let settings = Row::new(pad, "settings", ctx.styles.small(Role::Faint)).mark(Mark::Settings);
    list(ctx, footer, &[settings], None);
}

/// One session as the rail shows it.
struct Entry {
    session: SessionId,
    title: String,
    mark: Mark,
    state: String,
    role: Role,
}

/// The row's second line and its role: state is colour, nothing else is.
fn status_of(app: &AppState, open: &Open) -> (String, Role) {
    let Some(activity) = app.agent.activity(&open.session.id) else {
        return ("idle".into(), Role::Ghost);
    };
    let label = match &activity.status {
        _ if !activity.asks.is_empty() => "asks".into(),
        AgentStatus::Working => "working".into(),
        AgentStatus::Done { .. } => "done".into(),
        AgentStatus::Idle => "idle".into(),
        AgentStatus::Exited { code } => format!("exited {code}"),
        AgentStatus::Error { message } => format!("error: {message}"),
    };
    let role = match activity.class() {
        AttentionClass::NeedsYou => Role::Attention,
        AttentionClass::Moving => Role::Working,
        AttentionClass::ActWhenYouLook => Role::Text,
        AttentionClass::Quiet => Role::Ghost,
    };
    (label, role)
}
