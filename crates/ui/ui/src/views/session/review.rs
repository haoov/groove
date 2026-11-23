//! The Diff tab while its session waits on a cluster write: the ask on a band, then the change it would make.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{ApprovalId, Ask};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::Panel;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::Button;

/// The ask over the diff, which holds the write's change once it is read.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, body: Rect) {
    let Some(ask) = waiting(app) else {
        return super::diff::draw(ctx, app, ui, body);
    };
    let tall = Spec::default().height(ctx.tokens.row * 2.0 + ctx.tokens.md * 2.0);
    let [band, rest] = column_in(body, [tall, Spec::fill()]);
    let shown = app.workspace.reviewed() == Some(ask.id.as_str());
    asking(ctx, band, &ask, shown);
    if shown {
        return super::diff::draw(ctx, app, ui, rest);
    }
    let room = rest.pad(Edges::all(ctx.tokens.md));
    let [line, _] = column_in(room, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    Label::new("Reading the change…", ctx.styles.body(Role::Faint)).draw(ctx, line);
}

/// The cluster write the selected session waits on.
fn waiting(app: &AppState) -> Option<Ask> {
    let session = app.session.selected.as_ref()?;
    let asks = &app.agent.activity(session)?.asks;
    asks.iter().find(|one| one.reviewed).cloned()
}

/// The band on the attention tint: what the write asks, then what it acts on, and its answers
/// once the change is `shown`.
fn asking(ctx: &mut Ctx, band: Rect, ask: &Ask, shown: bool) {
    let tint = ctx.styles.band_tint(Role::Attention);
    Panel::default().ground(tint).draw(ctx, band);
    groove_ui_kit::shape::hairline(ctx, band, ctx.styles.band_edge(Role::Attention));
    let row = Spec::default().height(ctx.tokens.row);
    let room = band.pad(Edges::all(ctx.tokens.md));
    let [first, second] = column_in(room, [row, row]);
    let asks = match ask.op.as_str() {
        groove_controllers::agent_service::tools::APPLY_EDIT => {
            groove_controllers::agent_service::tools::verb(&ask.op).to_uppercase()
        }
        op => crate::views::rail::asks_to(op).to_uppercase(),
    };
    Label::new(&asks, ctx.styles.heading(Role::Attention)).draw(ctx, first);
    let mut second = second;
    if shown {
        answers(ctx, &mut second, &ask.id);
    }
    Label::new(&ask.subject, ctx.styles.code(Role::Text)).draw(ctx, second);
}

/// Approve in peach, and refuse, from the right of `room`: bordered words in capitals.
fn answers(ctx: &mut Ctx, room: &mut Rect, id: &ApprovalId) {
    let gap = ctx.tokens.sm;
    let (ground, peach) = (
        ctx.styles.band_tint(Role::Attention),
        ctx.styles.band_edge(Role::Attention),
    );
    let approve = Button::new(
        "APPROVE",
        Target::Approve(id.clone()),
        Role::Attention,
        ground,
    );
    approve.edged(peach).right(ctx, room, gap);
    let raised = ctx.styles.panel_head();
    let refuse = Button::new("REFUSE", Target::Refuse(id.clone()), Role::Muted, raised);
    refuse.right(ctx, room, gap);
}
