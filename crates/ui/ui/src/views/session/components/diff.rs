//! The diff tab: the open file as rows, its two gutters and its colours.

use std::collections::BTreeMap;

use groove_controllers::AppState;
use groove_controllers::workspace_service::Opened;
use groove_gfx::Rect;
use groove_types::{DiffView, Highlight, Row, RowKind};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::{Mark, Role};
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
    header(ctx, head, app, ui, &file.path);
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

/// The file's path, what it changed, and which view it is drawn in.
fn header(ctx: &mut Ctx, band: Rect, app: &AppState, ui: &Ui, path: &str) {
    let rule = ctx.styles.line();
    let switched = switch(ctx, band, ui.session.view);
    let left = Rect::new(band.x, band.y, switched - band.x, band.h);
    let counts = counted(app, path);
    let at = counts
        .map(|counts| written(ctx, left, counts))
        .unwrap_or(left.right());
    let style = ctx.styles.code(Role::Muted);
    let room = (at - band.x - ctx.tokens.md * 2.0).max(0.0);
    let text = elide_start(ctx, path, &style, room);
    row(
        ctx,
        Rect::new(band.x, band.y, at - band.x, band.h),
        ctx.tokens.md,
        &text,
        style,
    );
    hairline(ctx, band, rule);
}

/// The three views, the current one raised. Returns where they start.
fn switch(ctx: &mut Ctx, line: Rect, current: DiffView) -> f32 {
    let (pad, gap) = (ctx.tokens.sm, ctx.tokens.xs);
    let mut at = line.right();
    for view in DiffView::ALL.into_iter().rev() {
        let role = match view == current {
            true => Role::Text,
            false => Role::Faint,
        };
        let style = ctx.styles.small(role);
        let width = ctx.measure(view.label(), &style) + pad * 2.0;
        at -= width;
        let box_ = Rect::new(at, line.y, width, line.h);
        if view == current {
            let raised = ctx.styles.raised();
            ctx.quad(box_, raised);
        }
        row(ctx, box_, pad, view.label(), style);
        ctx.hit(box_, Target::View(view));
        at -= gap;
    }
    at
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
    match ui.session.view {
        DiffView::Split => beside(ctx, body, file, ui),
        view => {
            let rows = drawn(file, ui, view, Side::New);
            surface(ctx, body, &rows, ui, true);
        }
    }
}

/// The old on the left, the new on the right, one alignment between them.
fn beside(ctx: &mut Ctx, body: Rect, file: &Opened, ui: &Ui) {
    let thickness = ctx.tokens.hairline;
    let half = ((body.w - thickness) / 2.0).floor();
    let left = Rect::new(body.x, body.y, half, body.h);
    let right = Rect::new(
        left.right() + thickness,
        body.y,
        body.w - half - thickness,
        body.h,
    );
    let rule = ctx.styles.line();
    ctx.quad(Rect::new(left.right(), body.y, thickness, body.h), rule);
    surface(
        ctx,
        left,
        &drawn(file, ui, DiffView::Split, Side::Old),
        ui,
        false,
    );
    surface(
        ctx,
        right,
        &drawn(file, ui, DiffView::Split, Side::New),
        ui,
        true,
    );
}

/// Which file a row is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Old,
    New,
}

/// A row as the surface needs it, owned so the lines can borrow it.
struct Drawn {
    text: String,
    spans: Vec<Highlight>,
    gutters: Vec<String>,
    kind: RowKind,
    caret: bool,
    /// What the file view says happened to this line.
    mark: Option<Mark>,
}

/// The rows of one view, read from one side.
fn drawn(file: &Opened, ui: &Ui, view: DiffView, side: Side) -> Vec<Drawn> {
    let marks = marks(&file.rows);
    let file_view = view == DiffView::File;
    file.rows
        .iter()
        .enumerate()
        .filter(|(_, row)| shown(row, view, side))
        .map(|(at, row)| {
            let (text, spans) = line(file, row, source(row, view, side));
            Drawn {
                text,
                spans,
                gutters: gutters(row, view, side),
                kind: match file_view {
                    true => RowKind::Context,
                    false => kind(row, view, side),
                },
                caret: ui.session.at.is_some_and(|(caret, _)| caret == at),
                mark: file_view.then(|| marks.get(&row.new?).copied()).flatten(),
            }
        })
        .collect()
}

/// What each line of the new file did, by walking the changes either side of it.
/// A line both removed and added changed in place; a removal with nothing in its
/// place marks the line that closed the gap.
fn marks(rows: &[Row]) -> BTreeMap<u32, Mark> {
    let mut marks = BTreeMap::new();
    let mut at = 0;
    while at < rows.len() {
        let gone: Vec<&Row> = rows[at..]
            .iter()
            .take_while(|row| row.kind == RowKind::Removed)
            .collect();
        let came: Vec<&Row> = rows[at + gone.len()..]
            .iter()
            .take_while(|row| row.kind == RowKind::Added)
            .collect();
        if gone.is_empty() && came.is_empty() {
            at += 1;
            continue;
        }
        let mark = match (gone.is_empty(), came.is_empty()) {
            (false, false) => Mark::Changed,
            (true, false) => Mark::Added,
            _ => Mark::Removed,
        };
        match came.is_empty() {
            true => {
                let next = rows[at + gone.len()..].iter().find_map(|row| row.new);
                marks.extend(next.map(|line| (line, mark)));
            }
            false => marks.extend(came.iter().filter_map(|row| row.new).map(|l| (l, mark))),
        }
        at += gone.len() + came.len();
    }
    marks
}

/// The file view shows what is there now; the others show every row.
fn shown(row: &Row, view: DiffView, _side: Side) -> bool {
    match view {
        DiffView::File => row.new.is_some(),
        _ => true,
    }
}

/// Which file a row's line is read from: a pane's own side in split, and in the
/// other views whichever side the row belongs to.
fn source(row: &Row, view: DiffView, side: Side) -> Side {
    match (view, row.kind) {
        (DiffView::Split, _) => side,
        (_, RowKind::Removed) => Side::Old,
        _ => Side::New,
    }
}

/// In split a row with nothing on this side draws blank and carries no ground.
fn kind(row: &Row, view: DiffView, side: Side) -> RowKind {
    match (view, row.kind, side) {
        (DiffView::Split, RowKind::Added, Side::Old) => RowKind::Context,
        (DiffView::Split, RowKind::Removed, Side::New) => RowKind::Context,
        _ => row.kind,
    }
}

/// Where the line sits: one number a side in split, otherwise one row's own.
fn gutters(row: &Row, view: DiffView, side: Side) -> Vec<String> {
    let number = |at: Option<u32>| at.map(|at| (at + 1).to_string()).unwrap_or_default();
    match (view, row.kind) {
        (_, RowKind::Gap(_)) => Vec::new(),
        (DiffView::Split, _) => match side {
            Side::Old => vec![number(row.old)],
            Side::New => vec![number(row.new)],
        },
        (DiffView::File, _) => vec![number(row.new)],
        (DiffView::Inline, RowKind::Removed) => vec![number(row.old), String::new()],
        (DiffView::Inline, _) => vec![String::new(), number(row.new)],
    }
}

/// Draws the rows, clamps the scroll, and says where a click can land.
fn surface(ctx: &mut Ctx, rect: Rect, rows: &[Drawn], ui: &Ui, clickable: bool) {
    let gutters: Vec<Vec<&str>> = rows
        .iter()
        .map(|row| row.gutters.iter().map(String::as_str).collect())
        .collect();
    let lines: Vec<Line<'_>> = rows
        .iter()
        .enumerate()
        .map(|(at, row)| {
            if let RowKind::Gap(_) = row.kind {
                return Line::banner(&row.text);
            }
            let line = Line::new(&row.text).gutters(&gutters[at]).spans(&row.spans);
            let line = line.mark(row.mark.map(|mark| ctx.styles.mark(mark)));
            match ctx.styles.row_ground(row.kind, row.caret) {
                Some(color) => line.ground(color),
                None => line,
            }
        })
        .collect();
    let scroll = ui
        .session
        .diff
        .min((height(ctx, lines.len()) - rect.h).max(0.0));
    if clickable {
        ctx.hit(rect, Target::Code);
    }
    code(ctx, rect, &lines, scroll);
}

/// The row's line and its colours. The ground says whether it came or went.
fn line(file: &Opened, row: &Row, side: Side) -> (String, Vec<Highlight>) {
    if let RowKind::Gap(lines) = row.kind {
        return (format!("\u{2026} {lines} lines"), Vec::new());
    }
    let (document, at) = match side {
        Side::Old => (&file.old, row.old),
        Side::New => (&file.new, row.new),
    };
    let Some(at) = at.map(|at| at as usize) else {
        return (String::new(), Vec::new());
    };
    let text = document.line(at).unwrap_or_default();
    (text.to_string(), document.spans(at))
}
