//! The terminals' drawer under the strip, in glass: their bar alone while folded, then their grids.

use groove_controllers::AppState;
use groove_controllers::shell_service::{Shell, Shells};
use groove_gfx::{Edges, Rect};
use groove_types::SessionId;
use groove_ui_kit::shape::{from_bottom, from_right, from_top, strip};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, Tab, fold_turn, glass_pane, screen};

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, session: &SessionId) {
    let rect = ctx.app.layout.manual;
    if rect.is_empty() {
        return;
    }
    let shells = app.shell.shells(session);
    if !ui.session.manual {
        return folded(ctx, rect, shells);
    }
    glass_pane(ctx, rect);
    let mut body = rect;
    bar(ctx, from_top(&mut body, ctx.tokens.bar, 0.0), ui, shells);
    let Some(shells) = shells.filter(|one| !one.tabs.is_empty()) else {
        return said(
            ctx,
            body,
            "no terminal yet: + opens one in the session's directory",
        );
    };
    let shown = shells.shown();
    let panes = ctx.app.layout.shell_panes(&ctx.tokens, shown.len());
    let typing = ui.focus == crate::Focus::Terminal;
    for (shell, pane) in shown.iter().zip(panes) {
        let focused = typing && shells.focused() == Some(shell.id);
        grid(ctx, pane, shell, (focused, shown.len() > 1));
    }
}

/// The fold, the section's name, a tab each, then what opens a tab and splits the one up.
fn bar(ctx: &mut Ctx, line: Rect, ui: &Ui, shells: Option<&Shells>) {
    let (sm, md) = (ctx.tokens.sm, ctx.tokens.md);
    let mut room = line.pad(Edges::across(sm, sm));
    actions(ctx, &mut room);
    folder(ui.session.manual).left(ctx, &mut room, md);
    let named = Label::new("terminals", ctx.styles.label(Role::Muted));
    named.left(ctx, &mut room, md);
    for (name, id, selected) in tabs(shells) {
        tab(&name, id, selected).left(ctx, &mut room, ctx.tokens.xs);
    }
}

/// The bar alone, as narrow as what it holds, at the drawer's right end.
fn folded(ctx: &mut Ctx, line: Rect, shells: Option<&Shells>) {
    let (xs, md) = (ctx.tokens.xs, ctx.tokens.md);
    let mut room = line.pad(Edges::across(0.0, ctx.tokens.sm));
    actions(ctx, &mut room);
    let held = tabs(shells);
    for (name, id, selected) in held.iter().rev() {
        tab(name, *id, *selected).right(ctx, &mut room, xs);
    }
    if !held.is_empty() {
        from_right(&mut room, md - xs, 0.0);
    }
    let named = Label::new("terminals", ctx.styles.label(Role::Muted));
    named.right(ctx, &mut room, md);
    let slot = folder(false).right(ctx, &mut room, 0.0);
    let left = slot.x - ctx.tokens.sm;
    glass_pane(ctx, strip(line, left - line.x, line.right() - left));
}

/// The caret that folds the drawer or opens it.
fn folder(open: bool) -> Button<'static, Target> {
    Button::icon(Mark::Down, fold_turn(open), Target::ShellFold, Role::Faint)
}

/// What splits the selected tab and what opens a new one, at the right of `room`.
fn actions(ctx: &mut Ctx, room: &mut Rect) {
    let (ground, hover) = (ctx.styles.ground(), ctx.styles.glass_hover());
    let splits = Button::new("split", Target::ShellSplit, Role::Muted, ground);
    splits.flat().hover(hover).right(ctx, room, ctx.tokens.xs);
    let new = Button::icon(Mark::Add, 0, Target::ShellNew, Role::Muted);
    new.right(ctx, room, ctx.tokens.sm);
}

/// Each tab's name, the numbers of its terminals, its id, and whether it is up.
fn tabs(shells: Option<&Shells>) -> Vec<(String, u64, bool)> {
    let Some(shells) = shells else {
        return Vec::new();
    };
    shells
        .tabs
        .iter()
        .map(|one| {
            let numbers: Vec<String> = one
                .panes
                .iter()
                .map(|pane| shells.number(pane.id).to_string())
                .collect();
            let name = format!("term {}", numbers.join(" · "));
            (name, one.id, shells.selected == Some(one.id))
        })
        .collect()
}

/// One tab, a rounded row on the glass, and the cross that ends its terminals.
fn tab(name: &str, id: u64, selected: bool) -> Tab<'_, Target> {
    Tab::new(name, Target::ShellTab(id), selected)
        .glass()
        .close(Mark::Close, Target::ShellCloseTab(id))
}

/// One terminal's grid, or what stands in its place; a pane of several can close alone.
fn grid(ctx: &mut Ctx, pane: Rect, shell: &Shell, (focused, closable): (bool, bool)) {
    ctx.hit(pane, Target::Shell(shell.id));
    let inner = pane.pad(Edges::all(ctx.tokens.sm));
    let origin = (inner.x, inner.y);
    match (&shell.terminal, &shell.failed) {
        (Some(terminal), _) => screen(ctx, pane, origin, (&terminal.screen(), focused)),
        (None, Some(why)) => said(ctx, pane, why),
        (None, None) => said(ctx, pane, "starting the shell…"),
    }
    if let Some(code) = shell.exited {
        let mut foot = pane;
        let line = from_bottom(&mut foot, ctx.tokens.row, 0.0);
        let room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        let words = format!("exited {code}");
        Label::new(&words, ctx.styles.small(Role::Faint)).draw(ctx, room);
    }
    if closable {
        let mut corner = pane;
        let top = from_top(&mut corner, ctx.tokens.row, 0.0);
        let mut room = top.pad(Edges::across(0.0, ctx.tokens.xs));
        ctx.layer();
        let close = Button::icon(Mark::Close, 0, Target::ShellClose(shell.id), Role::Faint);
        close.right(ctx, &mut room, 0.0);
    }
}

fn said(ctx: &mut Ctx, pane: Rect, text: &str) {
    let mut room = pane.pad(Edges::all(ctx.tokens.md));
    let line = from_top(&mut room, ctx.tokens.row, 0.0);
    Label::new(text, ctx.styles.body(Role::Faint)).draw(ctx, line);
}
