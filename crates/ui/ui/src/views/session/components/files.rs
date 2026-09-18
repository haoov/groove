//! The sidebar's files tab: what changed in the selected worktree, the common root
//! once and then a group per directory under it.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::FileDiff;

use super::commit;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::style::Role;
use crate::widget::{button, elide, hairline, row, ruled};

pub(crate) struct Listing<'a> {
    pub groups: Vec<Group<'a>>,
}

pub(crate) struct Group<'a> {
    pub dir: String,
    pub files: Vec<&'a FileDiff>,
}

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let rect = ctx.layout.sidebar;
    if rect.is_empty() {
        return;
    }
    let panel = ctx.styles.panel();
    ctx.quad(rect, panel);
    edge(ctx, rect);

    let files = changed(app);
    let listing = listing(files);
    let head = Rect::new(rect.x, rect.y, rect.w, ctx.tokens.header);
    heading(ctx, head, files.len());
    let under = ctx.layout.commit;
    let body = Rect::new(rect.x, head.bottom(), rect.w, under.y - head.bottom());
    commit::draw(ctx, app, ui, under);
    if files.is_empty() {
        let style = ctx.styles.small(Role::Faint);
        let line = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
        return row(ctx, line, ctx.tokens.md, "nothing changed", style);
    }
    let open = app.workspace.opened.as_ref().map(|file| &file.path);
    rows(ctx, body, &listing, open, ui);
}

fn edge(ctx: &mut Ctx, rect: Rect) {
    let (rule, thickness) = (ctx.styles.line(), ctx.tokens.hairline);
    ctx.quad(Rect::new(rect.x, rect.y, thickness, rect.h), rule);
}

fn heading(ctx: &mut Ctx, rect: Rect, count: usize) {
    let (rule, pad) = (ctx.styles.line(), ctx.tokens.md);
    let style = ctx.styles.heading(Role::Faint);
    let label = match count {
        0 => "FILES".to_string(),
        n => format!("FILES · {n}"),
    };
    row(ctx, rect, pad, &label, style);
    hairline(ctx, rect, rule);
}

fn rows(ctx: &mut Ctx, body: Rect, listing: &Listing<'_>, open: Option<&String>, ui: &Ui) {
    let scroll = ui.session.files;
    let height = ctx.tokens.row;
    let lines = listing
        .groups
        .iter()
        .map(|g| g.files.len() + 1)
        .sum::<usize>();
    let extent = (height * lines as f32 - body.h).max(0.0);
    ctx.scrolls(Scroller::Files, extent);
    let scroll = scroll.min(extent);
    ctx.clipped(body, |ctx| {
        let mut y = body.y - scroll;
        for group in &listing.groups {
            if !group.dir.is_empty() {
                let style = ctx.styles.small(Role::Faint);
                let line = Rect::new(body.x, y, body.w, height);
                let text = elide(ctx, &group.dir, &style, body.w - ctx.tokens.md * 2.0);
                row(ctx, line, ctx.tokens.md, &text, style);
                y += height;
            }
            for file in &group.files {
                let line = Rect::new(body.x, y, body.w, height);
                entry(
                    ctx,
                    line,
                    file,
                    group.dir.is_empty(),
                    open == Some(&file.path),
                    ui,
                );
                y += height;
            }
        }
    });
}

fn entry(ctx: &mut Ctx, line: Rect, file: &FileDiff, at_root: bool, open: bool, ui: &Ui) {
    if ui.discarding.as_deref() == Some(file.path.as_str()) {
        return asking(ctx, line, &file.path, ui);
    }
    let on_row = pointed(ui, &file.path);
    if on_row {
        let hover = ctx.styles.hover();
        ctx.quad(line, hover);
    }
    if open {
        let here = ctx.styles.here();
        ruled(ctx, line, here);
    }
    let letter = ctx.styles.small(Role::Ghost);
    let indent = match at_root {
        true => ctx.tokens.md,
        false => ctx.tokens.md + ctx.tokens.md,
    };
    let mark = file.status.letter().to_string();
    row(ctx, line, indent, &mark, letter);

    ctx.hit(line, Target::File(file.path.clone()));
    let at = match on_row {
        true => offer(ctx, line, file, ui),
        false => counts(ctx, line, file),
    };
    let name = ctx.styles.body(Role::Text);
    let start = indent + ctx.tokens.md;
    let room = (at - line.x - start - ctx.tokens.sm).max(0.0);
    let text = elide(ctx, name_of(&file.path), &name, room);
    row(
        ctx,
        Rect::new(line.x, line.y, at - line.x, line.h),
        start,
        &text,
        name,
    );
}

/// Whether the pointer is on this row, or on what the row is offering.
fn pointed(ui: &Ui, path: &str) -> bool {
    match &ui.hover {
        Some(Target::File(at) | Target::Stage(at) | Target::Unstage(at)) => at == path,
        _ => false,
    }
}

/// What the row offers the pointer. Returns where it starts.
fn offer(ctx: &mut Ctx, line: Rect, file: &FileDiff, ui: &Ui) -> f32 {
    let staged = file.staged == Some(true);
    let (label, target) = match staged {
        true => ("unstage", Target::Unstage(file.path.clone())),
        false => ("stage", Target::Stage(file.path.clone())),
    };
    acted(ctx, line, label, target, ui)
}

/// One word at the end of a row, with a ground of its own under the pointer.
fn acted(ctx: &mut Ctx, line: Rect, label: &str, target: Target, ui: &Ui) -> f32 {
    let style = ctx.styles.small(Role::Muted);
    let on_it = ui.hover.as_ref() == Some(&target);
    let ground = on_it.then(|| ctx.styles.action());
    let box_ = button(ctx, line, label, style, ground);
    ctx.hit(box_, target);
    box_.x
}

/// The row asks before it throws a change away, in the row's own place.
fn asking(ctx: &mut Ctx, line: Rect, path: &str, ui: &Ui) {
    ctx.quad(line, ctx.styles.raised());
    let keep = acted(ctx, line, "keep", Target::Keep, ui);
    let gone = Rect::new(line.x, line.y, keep - line.x, line.h);
    let discard = acted(ctx, gone, "discard", Target::Discard(path.to_string()), ui);
    let style = ctx.styles.small(Role::Bad);
    let asked = Rect::new(line.x, line.y, discard - line.x, line.h);
    let text = elide(
        ctx,
        "discard changes?",
        &style,
        asked.w - ctx.tokens.md * 2.0,
    );
    row(ctx, asked, ctx.tokens.md, &text, style);
}

/// Returns where the counts start.
fn counts(ctx: &mut Ctx, line: Rect, file: &FileDiff) -> f32 {
    let mut at = line.right() - ctx.tokens.md;
    for (count, role) in [(file.deleted, Role::Bad), (file.added, Role::Ok)] {
        if count == 0 {
            continue;
        }
        let style = ctx.styles.small(role);
        let sign = match role {
            Role::Ok => '+',
            _ => '-',
        };
        let text = format!("{sign}{count}");
        let width = ctx.measure(&text, &style);
        at -= width;
        row(ctx, Rect::new(at, line.y, width, line.h), 0.0, &text, style);
        at -= ctx.tokens.sm;
    }
    at
}

/// The files of the worktree the session points at, never another's.
pub(crate) fn changed(app: &AppState) -> &[FileDiff] {
    let selected = app
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|worktree| &worktree.id);
    app.workspace.files_of(selected)
}

fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub(crate) fn listing(files: &[FileDiff]) -> Listing<'_> {
    let mut groups: Vec<Group<'_>> = Vec::new();
    for file in files {
        let dir = segments(&file.path).join("/");
        match groups.iter_mut().find(|group| group.dir == dir) {
            Some(group) => group.files.push(file),
            None => groups.push(Group {
                dir,
                files: vec![file],
            }),
        }
    }
    groups.sort_by(|a, b| a.dir.cmp(&b.dir));
    Listing { groups }
}

/// A path's directories, without its file name.
fn segments(path: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = path.split('/').collect();
    parts.pop();
    parts
}
