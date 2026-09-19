//! The sidebar's files tab: what changed in the selected worktree, the common root
//! once and then a group per directory under it.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::FileDiff;

use super::commit;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{button, elide, hairline, row, ruled};
use crate::{Losing, Ui};

pub(crate) struct Listing<'a> {
    /// What every file has in common, shown once above them.
    pub root: String,
    pub groups: Vec<Group<'a>>,
}

pub(crate) struct Group<'a> {
    /// The directory under the root, its single-child chains collapsed.
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

    let files = narrowed(app, ui);
    let listing = listing(&files);
    let bar = Rect::new(rect.x, rect.y, rect.w, ctx.tokens.row);
    searching(ctx, bar, ui);
    let head = Rect::new(rect.x, bar.bottom(), rect.w, ctx.tokens.header);
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

/// The changed files the search bar leaves, every one of them when it is empty.
pub(crate) fn narrowed<'a>(app: &'a AppState, ui: &Ui) -> Vec<&'a FileDiff> {
    let files: Vec<&FileDiff> = changed(app).iter().collect();
    match ui.session.query.is_empty() {
        true => files,
        false => {
            let query = ui.session.query.text();
            crate::palette::matching(files, |file| file.path.clone(), query)
        }
    }
}

/// The bar the search types into, above the list it narrows.
fn searching(ctx: &mut Ctx, rect: Rect, ui: &Ui) {
    ctx.quad(rect, ctx.styles.ground());
    hairline(ctx, rect, ctx.styles.line());
    let (text, role) = match (ui.session.searching, ui.session.query.is_empty()) {
        (false, true) => ("filter by path".to_string(), Role::Ghost),
        (true, _) => (ui.session.query.shown(), Role::Text),
        (false, _) => (ui.session.query.text().to_string(), Role::Text),
    };
    let size = ctx.tokens.icon;
    let glass = Rect::new(
        rect.x + ctx.tokens.md,
        rect.y + (rect.h - size) / 2.0,
        size,
        size,
    );
    ctx.icon(glass, Mark::Search, 0, ctx.styles.color(role));
    let at = glass.right() - rect.x + ctx.tokens.sm;
    row(ctx, rect, at, &text, ctx.styles.code(role));
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
    let root = usize::from(!listing.root.is_empty());
    let lines = root
        + listing
            .groups
            .iter()
            .map(|g| g.files.len() + usize::from(!g.dir.is_empty()))
            .sum::<usize>();
    let extent = (height * lines as f32 - body.h).max(0.0);
    ctx.scrolls(Scroller::Files, extent);
    let scroll = scroll.min(extent);
    ctx.clipped(body, |ctx| {
        let mut y = body.y - scroll;
        if !listing.root.is_empty() {
            let style = ctx.styles.small(Role::Ghost);
            let line = Rect::new(body.x, y, body.w, height);
            let room = body.w - ctx.tokens.md * 2.0;
            let text = elide(ctx, &listing.root, &style, room);
            row(ctx, line, ctx.tokens.md, &text, style);
            y += height;
        }
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
    if ui.discarding == Some(Losing::File(file.path.clone())) {
        return asking(ctx, line, "discard changes?", ui);
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
    let start = indent + ctx.tokens.md;
    let room = (at - line.x - start - ctx.tokens.sm).max(0.0);
    named(
        ctx,
        Rect::new(line.x, line.y, at - line.x, line.h),
        file,
        start,
        room,
    );
}

/// The file's name, then what is left of its path behind it, dimmed.
fn named(ctx: &mut Ctx, line: Rect, file: &FileDiff, start: f32, room: f32) {
    let (name, behind) = reads_as(&file.path);
    let strong = ctx.styles.body(Role::Text);
    let quiet = ctx.styles.small(Role::Ghost);
    let text = elide(ctx, &name, &strong, room);
    let width = ctx.measure(&text, &strong);
    row(ctx, line, start, &text, strong);
    let left = room - width - ctx.tokens.sm;
    if behind.is_empty() || left <= 0.0 {
        return;
    }
    let rest = elide(ctx, &behind, &quiet, left);
    let at = line.x + start + width + ctx.tokens.sm;
    row(ctx, Rect::new(at, line.y, left, line.h), 0.0, &rest, quiet);
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
pub(crate) fn acted(ctx: &mut Ctx, line: Rect, label: &str, target: Target, ui: &Ui) -> f32 {
    let style = ctx.styles.small(Role::Muted);
    let on_it = ui.hover.as_ref() == Some(&target);
    let ground = on_it.then(|| ctx.styles.action());
    let box_ = button(ctx, line, label, style, ground);
    ctx.hit(box_, target);
    box_.x
}

/// A question in the row's own place, with its two answers at its end.
pub(crate) fn asking(ctx: &mut Ctx, line: Rect, question: &str, ui: &Ui) {
    ctx.quad(line, ctx.styles.raised());
    let keep = acted(ctx, line, "keep", Target::Keep, ui);
    let gone = Rect::new(line.x, line.y, keep - line.x, line.h);
    let discard = acted(ctx, gone, "discard", Target::Discard, ui);
    let style = ctx.styles.small(Role::Bad);
    let asked = Rect::new(line.x, line.y, discard - line.x, line.h);
    let room = (asked.w - ctx.tokens.md * 2.0).max(0.0);
    let text = elide(ctx, question, &style, room);
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

pub(crate) fn listing<'a>(files: &[&'a FileDiff]) -> Listing<'a> {
    let root = common(files);
    let mut groups: Vec<Group<'a>> = Vec::new();
    for file in files {
        let dir = under(&root, &file.path);
        match groups.iter_mut().find(|group| group.dir == dir) {
            Some(group) => group.files.push(file),
            None => groups.push(Group {
                dir,
                files: vec![file],
            }),
        }
    }
    groups.sort_by(|a, b| a.dir.cmp(&b.dir));
    Listing { root, groups }
}

/// The longest path every file shares, and nothing when they share none.
fn common(files: &[&FileDiff]) -> String {
    let mut shared: Option<Vec<&str>> = None;
    for file in files {
        let parts = segments(&file.path);
        shared = Some(match shared {
            None => parts,
            Some(shared) => shared
                .iter()
                .zip(parts.iter())
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| *a)
                .collect(),
        });
    }
    shared.unwrap_or_default().join("/")
}

/// A file's directory with the root taken off its front.
fn under(root: &str, path: &str) -> String {
    let dir = segments(path).join("/");
    match dir.strip_prefix(root) {
        Some(rest) => rest.trim_start_matches('/').to_string(),
        None => dir,
    }
}

/// The name a file reads as, and the path behind it. `mod.rs` and its like read as
/// the directory that holds them.
pub(crate) fn reads_as(path: &str) -> (String, String) {
    let name = name_of(path);
    let parts = segments(path);
    let plain = [
        "mod.rs", "lib.rs", "main.rs", "index.ts", "index.js", "mod.ts",
    ];
    match plain.contains(&name) {
        true => match parts.last() {
            Some(dir) => (format!("{dir}/{name}"), parts[..parts.len() - 1].join("/")),
            None => (name.to_string(), String::new()),
        },
        false => (name.to_string(), parts.join("/")),
    }
}

/// A path's directories, without its file name.
fn segments(path: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = path.split('/').collect();
    parts.pop();
    parts
}
