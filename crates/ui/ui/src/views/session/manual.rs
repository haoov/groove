//! The manual section: the session's own terminals under the tab, their bar and their grids.

use groove_controllers::AppState;
use groove_controllers::shell_service::{Shell, Shells};
use groove_gfx::{Edges, Rect};
use groove_types::SessionId;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hoverable;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, Tab, fold, screen};

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, session: &SessionId) {
    let rect = ctx.app.layout.manual;
    if rect.is_empty() {
        return;
    }
    groove_ui_kit::shape::ground(ctx, rect, Ground::Band);
    groove_ui_kit::shape::top_rule(ctx, rect, ctx.styles.line());
    let shells = app.shell.shells(session);
    let mut body = rect;
    bar(ctx, body.take_top(ctx.tokens.bar), ui, shells);
    if !ui.session.manual {
        return;
    }
    let Some(shells) = shells.filter(|one| !one.tabs.is_empty()) else {
        return said(
            ctx,
            body,
            "no terminal yet: + opens one in the session's directory",
        );
    };
    let shown = shells.shown();
    let panes = ctx.app.layout.shell_panes(&ctx.tokens, shown.len());
    let typing = ui.focus == crate::Focus::Terminal && ui.pane_has_keys();
    for (shell, pane) in shown.iter().zip(panes) {
        let focused = typing && shells.focused() == Some(shell.id);
        grid(ctx, pane, shell, (focused, shown.len() > 1));
    }
}

/// The fold, the section's name, a tab each, then what opens a tab and splits the one up.
fn bar(ctx: &mut Ctx, line: Rect, ui: &Ui, shells: Option<&Shells>) {
    let (sm, md) = (ctx.tokens.sm, ctx.tokens.md);
    let mut room = line.pad(Edges::across(0.0, sm));
    let ground = ctx.styles.ground();
    let splits = Button::new("split", Target::ShellSplit, Role::Muted, ground);
    splits.right(ctx, &mut room, ctx.tokens.xs);
    Button::new("+", Target::ShellNew, Role::Muted, ground).right(ctx, &mut room, sm);
    let mut folds = room.take_left(line.h);
    hoverable(ctx, folds, Target::ShellFold);
    folds.take_left((line.h - ctx.tokens.small) / 2.0);
    fold(ctx, &mut folds, ui.session.manual, Role::Faint);
    let named = Label::new("terminals", ctx.styles.label(Role::Muted));
    named.left(ctx, &mut room, md);
    let Some(shells) = shells else {
        return;
    };
    for one in &shells.tabs {
        let numbers: Vec<String> = one
            .panes
            .iter()
            .map(|pane| shells.number(pane.id).to_string())
            .collect();
        let name = format!("term {}", numbers.join(" · "));
        tab(
            ctx,
            &mut room,
            (&name, one.id),
            shells.selected == Some(one.id),
        );
    }
}

/// One tab: the numbers of its terminals, raised while up, and the cross that ends them all.
fn tab(ctx: &mut Ctx, room: &mut Rect, (name, id): (&str, u64), selected: bool) {
    let tab = Tab::new(name, Target::ShellTab(id), selected);
    tab.close(Mark::Close, Target::ShellCloseTab(id))
        .left(ctx, room, 0.0);
}

/// One terminal's grid, or what stands in its place; a pane of several can close alone.
fn grid(ctx: &mut Ctx, pane: Rect, shell: &Shell, (focused, closable): (bool, bool)) {
    groove_ui_kit::shape::ground(ctx, pane, Ground::Deep);
    ctx.hit(pane, Target::Shell(shell.id));
    let origin = (pane.x + ctx.tokens.sm, pane.y + ctx.tokens.sm);
    match (&shell.terminal, &shell.failed) {
        (Some(terminal), _) => screen(ctx, pane, origin, (&terminal.screen(), focused)),
        (None, Some(why)) => said(ctx, pane, why),
        (None, None) => said(ctx, pane, "starting the shell…"),
    }
    if let Some(code) = shell.exited {
        let mut foot = pane;
        let line = foot.take_bottom(ctx.tokens.row);
        let room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        let words = format!("exited {code}");
        Label::new(&words, ctx.styles.small(Role::Faint)).draw(ctx, room);
    }
    if closable {
        let mut corner = pane;
        let top = corner.take_top(ctx.tokens.row);
        let mut room = top.pad(Edges::across(0.0, ctx.tokens.xs));
        ctx.layer();
        let close = Button::icon(Mark::Close, 0, Target::ShellClose(shell.id), Role::Faint);
        close.right(ctx, &mut room, 0.0);
    }
}

fn said(ctx: &mut Ctx, pane: Rect, text: &str) {
    let mut room = pane.pad(Edges::all(ctx.tokens.md));
    let line = room.take_top(ctx.tokens.row);
    Label::new(text, ctx.styles.body(Role::Faint)).draw(ctx, line);
}
