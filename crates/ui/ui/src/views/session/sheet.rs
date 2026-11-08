//! The review sheet: one write shown whole over the work half, with its two answers.
//! A write that carries a change shows it as the diff does, read-only.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{ApprovalId, Ask, DiffView};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in, column_of};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, Tab, Text};

/// The write under review, over the workspace and the sidebar.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let Some((sheet, ask)) = ui.sheet().and_then(|one| Some((one, found(app, &one.id)?))) else {
        return;
    };
    ctx.layer();
    let area = area(ctx);
    groove_ui_kit::shape::ground(ctx, area, Ground::Work);
    ctx.hit(area, Target::Sheet);
    let tall = |height: f32| Spec::default().height(height);
    let tokens = ctx.tokens;
    let [content, foot] = column_in(area, [Spec::fill(), tall(tokens.row + 2.0 * tokens.md)]);
    groove_ui_kit::shape::top_rule(ctx, foot, ctx.styles.line());
    let parts = [
        tall(tokens.row),
        tall(tokens.row),
        tall(tokens.sm),
        Spec::fill(),
    ];
    let [first, second, _, body] = column_in(content.pad(Edges::all(tokens.md)), parts);
    let asks = crate::views::rail::asks_to(&ask.op);
    Label::new(&asks, ctx.styles.title(Role::Attention)).draw(ctx, first);
    let place = place(app, &ask);
    let review = app.workspace.reviews.get(&ask.id);
    let mut second = second;
    if review.is_some() {
        switch(ctx, &mut second, sheet.view);
    }
    Label::new(&place, ctx.styles.body(Role::Muted)).draw(ctx, second);
    match review {
        Some(review) => crate::views::session::diff::review::draw(ctx, ui, body, (review, sheet)),
        None => said(ctx, body, &ask.text),
    }
    answers(ctx, foot.pad(Edges::all(tokens.md)), &ask.id);
}

/// The workspace and the sidebar together, from the header down.
fn area(ctx: &Ctx) -> Rect {
    ctx.app.layout.work
}

/// The write this id names, in whichever session waits on it.
fn found(app: &AppState, id: &ApprovalId) -> Option<Ask> {
    app.agent
        .agents
        .iter()
        .flat_map(|(_, agent)| agent.activity.asks.iter())
        .find(|ask| &ask.id == id)
        .cloned()
}

/// The repo and the branch the write acts in, when it acts in a worktree.
fn place(app: &AppState, ask: &Ask) -> String {
    let worktree = ask.worktree.as_ref().and_then(|id| {
        app.session
            .open
            .iter()
            .flat_map(|open| open.worktrees.iter())
            .find(|one| &one.id == id)
    });
    match worktree {
        Some(one) => format!("{} · {}", one.repo.as_str(), one.branch),
        None => ask.subject.clone(),
    }
}

/// The text of the write, a line a row, cut where the sheet ends.
fn said(ctx: &mut Ctx, rect: Rect, text: &str) {
    let (style, line) = (ctx.styles.code(Role::Text), ctx.tokens.line);
    let room = (rect.h / line).floor().max(0.0) as usize;
    let rows = column_of(rect, &vec![Spec::default().height(line); room]);
    for (one, row) in text.lines().zip(rows) {
        Label::new(one, style).draw(ctx, row);
    }
}

/// The views of the change, from the right of `room`, the current one raised.
fn switch(ctx: &mut Ctx, room: &mut Rect, current: DiffView) {
    for view in DiffView::ALL.into_iter().rev() {
        let tab = Tab::new(view.label(), Target::SheetView(view), view == current);
        let tab = tab.text(Text::Body).quiet(Role::Faint).tight();
        tab.right(ctx, room, 0.0);
    }
}

/// Approve in peach, and refuse, from the sheet's end.
fn answers(ctx: &mut Ctx, line: Rect, id: &ApprovalId) {
    let (mut room, raised) = (line, ctx.styles.raised());
    let refuse = Button::new("Refuse", Target::Refuse(id.clone()), Role::Muted, raised);
    refuse.right(ctx, &mut room, ctx.tokens.xs);
    let approve = Button::new(
        "Approve",
        Target::Approve(id.clone()),
        Role::Attention,
        raised,
    );
    approve.right(ctx, &mut room, ctx.tokens.xs);
}
