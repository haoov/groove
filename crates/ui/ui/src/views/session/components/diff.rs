//! The diff tab: the open file as rows, its two gutters and its colours.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Opened;
use groove_gfx::Rect;
use groove_types::{Highlight, Row, RowKind};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::Role;
use crate::widget::{Line, code, elide_start, hairline, height, row};

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let Some(file) = app.workspace.opened.as_ref() else {
        return hint(ctx, app, area);
    };
    if file.long {
        let style = ctx.styles.body(Role::Faint);
        let line = Rect::new(area.x, area.y, area.w, ctx.tokens.row);
        return row(ctx, line, ctx.tokens.md, "Too long to show.", style);
    }
    let head = Rect::new(area.x, area.y, area.w, ctx.tokens.row);
    header(ctx, head, app, &file.path);
    let body = Rect::new(area.x, head.bottom(), area.w, area.h - head.h);
    rows(ctx, body, file, ui);
}

/// What the tab says with nothing open.
fn hint(ctx: &mut Ctx, app: &AppState, area: Rect) {
    let style = ctx.styles.body(Role::Faint);
    let files = super::files::changed(app).len();
    let text = match files {
        0 => "No change in this worktree.".to_string(),
        1 => "1 file changed. Open it in the sidebar.".to_string(),
        n => format!("{n} files changed. Open one in the sidebar."),
    };
    let line = Rect::new(area.x, area.y, area.w, ctx.tokens.row);
    row(ctx, line, ctx.tokens.md, &text, style);
}

/// The file's path and what it changed, pinned above its rows.
fn header(ctx: &mut Ctx, line: Rect, app: &AppState, path: &str) {
    let rule = ctx.styles.line();
    let counts = counted(app, path);
    let at = counts
        .map(|counts| written(ctx, line, counts))
        .unwrap_or(line.right());
    let style = ctx.styles.code(Role::Muted);
    let room = (at - line.x - ctx.tokens.md * 2.0).max(0.0);
    let text = elide_start(ctx, path, &style, room);
    row(
        ctx,
        Rect::new(line.x, line.y, at - line.x, line.h),
        ctx.tokens.md,
        &text,
        style,
    );
    hairline(ctx, line, rule);
}

/// What the file added and deleted, right of its path. Returns where they start.
fn written(ctx: &mut Ctx, line: Rect, counts: (u32, u32)) -> f32 {
    let mut at = line.right() - ctx.tokens.md;
    for (count, role, sign) in [(counts.1, Role::Bad, '-'), (counts.0, Role::Ok, '+')] {
        if count == 0 {
            continue;
        }
        let style = ctx.styles.code(role);
        let text = format!("{sign}{count}");
        let width = ctx.measure(&text, &style);
        at -= width;
        row(ctx, Rect::new(at, line.y, width, line.h), 0.0, &text, style);
        at -= ctx.tokens.sm;
    }
    at
}

/// The summary's counts for this path.
fn counted(app: &AppState, path: &str) -> Option<(u32, u32)> {
    super::files::changed(app)
        .iter()
        .find(|file| file.path == path)
        .map(|file| (file.added, file.deleted))
}

fn rows(ctx: &mut Ctx, body: Rect, file: &Opened, ui: &Ui) {
    let numbers: Vec<[String; 2]> = file.rows.iter().map(gutters).collect();
    let drawn: Vec<(String, Vec<Highlight>)> =
        file.rows.iter().map(|row| line(file, row)).collect();
    let lines: Vec<Line<'_>> = file
        .rows
        .iter()
        .enumerate()
        .map(|(at, row)| {
            if let RowKind::Gap(_) = row.kind {
                return Line::banner(&drawn[at].0);
            }
            let line = Line::new(&drawn[at].0)
                .gutters([&numbers[at][0], &numbers[at][1]])
                .spans(&drawn[at].1);
            let caret = ui.session.at.is_some_and(|(row, _)| row == at);
            match ctx.styles.row_ground(row.kind, caret) {
                Some(color) => line.ground(color),
                None => line,
            }
        })
        .collect();
    let scroll = ui
        .session
        .diff
        .min((height(ctx, lines.len()) - body.h).max(0.0));
    ctx.hit(body, Target::Code);
    code(ctx, body, &lines, scroll);
}

/// Where the line sits: on the left what went, on the right what is there now.
fn gutters(row: &Row) -> [String; 2] {
    let number = |at: Option<u32>| at.map(|at| (at + 1).to_string()).unwrap_or_default();
    match row.kind {
        RowKind::Removed => [number(row.old), String::new()],
        RowKind::Added | RowKind::Context => [String::new(), number(row.new)],
        RowKind::Gap(_) => [String::new(), String::new()],
    }
}

/// The row's line and its colours. The ground says whether it came or went.
fn line(file: &Opened, row: &Row) -> (String, Vec<Highlight>) {
    if let RowKind::Gap(lines) = row.kind {
        return (format!("\u{2026} {lines} lines"), Vec::new());
    }
    let (document, at) = match row.kind {
        RowKind::Removed => (&file.old, row.old),
        _ => (&file.new, row.new),
    };
    let Some(at) = at.map(|at| at as usize) else {
        return (String::new(), Vec::new());
    };
    let text = document.line(at).unwrap_or_default();
    (text.to_string(), document.spans(at))
}
