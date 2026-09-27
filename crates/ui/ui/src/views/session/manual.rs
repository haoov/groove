//! The manual section: the session's own terminals under the tab, their bar and their grids.

use groove_controllers::AppState;
use groove_controllers::shell_service::{Shell, Shells};
use groove_gfx::{Align, Edges, Rect};
use groove_types::SessionId;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{hoverable, square};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Word, screen};

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, session: &SessionId) {
    let rect = ctx.app.layout.manual;
    if rect.is_empty() {
        return;
    }
    ctx.quad(rect, ctx.styles.band());
    let rule = Rect {
        h: ctx.tokens.hairline,
        ..rect
    };
    ctx.quad(rule, ctx.styles.line());
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
    let typing = ui.focus == crate::Focus::Terminal;
    for (shell, pane) in shown.iter().zip(panes) {
        let focused = typing && shells.focused() == Some(shell.id);
        grid(ctx, pane, shell, (focused, shown.len() > 1));
    }
}

/// The fold, the section's name, a tab each, then what opens a tab and splits the one up.
fn bar(ctx: &mut Ctx, line: Rect, ui: &Ui, shells: Option<&Shells>) {
    let (sm, md, size) = (ctx.tokens.sm, ctx.tokens.md, ctx.tokens.icon);
    let mut room = line.pad(Edges::across(0.0, sm));
    let ground = ctx.styles.ground();
    let splits = Word::new("split", Target::ShellSplit, Role::Muted, ground);
    splits.right(ctx, &mut room, ctx.tokens.xs);
    Word::new("+", Target::ShellNew, Role::Muted, ground).right(ctx, &mut room, sm);
    let fold = room.take_left(line.h);
    hoverable(ctx, fold, Target::ShellFold);
    let turn = match ui.session.manual {
        true => 0,
        false => Mark::RIGHTWARDS,
    };
    let mark = fold.align((size, size), Align::Center, Align::Center);
    ctx.icon(mark, Mark::Down, turn, ctx.styles.color(Role::Faint));
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
    let (sm, md, size) = (ctx.tokens.sm, ctx.tokens.md, ctx.tokens.small);
    let role = match selected {
        true => Role::Text,
        false => Role::Muted,
    };
    let label = Label::new(name, ctx.styles.label(role));
    let box_ = room.take_left(label.width(ctx) + md * 2.0 + sm + size);
    if selected {
        ctx.quad(box_, ctx.styles.raised());
    }
    hoverable(ctx, box_, Target::ShellTab(id));
    let mut inside = box_.pad(Edges::across(md, md));
    let cross = square(inside.take_right(size), size);
    inside.take_right(sm);
    label.draw(ctx, inside);
    ctx.icon(cross, Mark::Close, 0, ctx.styles.color(Role::Faint));
    ctx.hit(cross, Target::ShellCloseTab(id));
}

/// One terminal's grid, or what stands in its place; a pane of several can close alone.
fn grid(ctx: &mut Ctx, pane: Rect, shell: &Shell, (focused, closable): (bool, bool)) {
    ctx.quad(pane, ctx.styles.deep());
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
        let size = ctx.tokens.small;
        let mut corner = pane.pad(Edges::all(ctx.tokens.xs));
        let cross = corner
            .take_right(size)
            .align((size, size), Align::Start, Align::Start);
        ctx.layer();
        ctx.icon(cross, Mark::Close, 0, ctx.styles.color(Role::Faint));
        ctx.hit(cross, Target::ShellClose(shell.id));
    }
}

fn said(ctx: &mut Ctx, pane: Rect, text: &str) {
    let mut room = pane.pad(Edges::all(ctx.tokens.md));
    let line = room.take_top(ctx.tokens.row);
    Label::new(text, ctx.styles.body(Role::Faint)).draw(ctx, line);
}
