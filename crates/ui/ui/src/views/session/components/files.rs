//! The sidebar's files tab: what changed in the selected worktree, the common root
//! once and then a group per directory under it.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::FileDiff;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::Role;
use crate::widget::{elide, hairline, row};

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
    let body = Rect::new(rect.x, head.bottom(), rect.w, rect.h - head.h);
    if files.is_empty() {
        let style = ctx.styles.small(Role::Faint);
        let line = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
        return row(ctx, line, ctx.tokens.md, "nothing changed", style);
    }
    rows(ctx, body, &listing, ui.session.files);
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

fn rows(ctx: &mut Ctx, body: Rect, listing: &Listing<'_>, scroll: f32) {
    let height = ctx.tokens.row;
    let lines = listing
        .groups
        .iter()
        .map(|g| g.files.len() + 1)
        .sum::<usize>();
    let scroll = scroll.min((height * lines as f32 - body.h).max(0.0));
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
                entry(ctx, line, file, group.dir.is_empty());
                y += height;
            }
        }
    });
}

fn entry(ctx: &mut Ctx, line: Rect, file: &FileDiff, at_root: bool) {
    let letter = ctx.styles.small(Role::Ghost);
    let indent = match at_root {
        true => ctx.tokens.md,
        false => ctx.tokens.md + ctx.tokens.md,
    };
    let mark = file.status.letter().to_string();
    row(ctx, line, indent, &mark, letter);

    ctx.hit(line, Target::File(file.path.clone()));
    let at = counts(ctx, line, file);
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
