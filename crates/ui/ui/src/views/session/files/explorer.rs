//! The worktree as a tree: the directories standing open, and the files under them.

use std::collections::{BTreeMap, BTreeSet};

use groove_gfx::Rect;
use groove_types::FileDiff;

use super::rows::{Reading, entry};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::mark::Mark;
use crate::style::Role;
use crate::views::session::Asked;
use crate::widget::{box_in, elide, row};

/// One row of the tree: a directory to open, or a file to read.
pub(crate) struct Row<'a> {
    pub depth: usize,
    pub path: String,
    pub name: String,
    pub dir: bool,
    /// The changed file it is, when the diff holds one for it.
    pub file: Option<&'a FileDiff>,
}

/// What every directory holds, by its own path from the worktree root.
#[derive(Default)]
struct Node {
    dirs: BTreeSet<String>,
    files: BTreeSet<String>,
}

/// The rows the open directories leave, and the row a new path is being named in.
pub(crate) fn rows<'a>(paths: &[FileDiff], changed: &[&'a FileDiff], ui: &Ui) -> Vec<Row<'a>> {
    let tree = tree(paths);
    let mut out = Vec::new();
    walk(&tree, "", 0, &ui.session.opened, changed, &mut out);
    if let Some(naming) = ui
        .session
        .naming
        .as_ref()
        .filter(|one| matches!(one.asked, Asked::File | Asked::Folder))
    {
        let (at, depth) = under(&out, &naming.at);
        out.insert(
            at,
            Row {
                depth,
                path: String::new(),
                name: String::new(),
                dir: naming.asked == Asked::Folder,
                file: None,
            },
        );
    }
    out
}

/// Where a new path's row goes: just inside the directory it belongs to.
fn under(rows: &[Row<'_>], dir: &str) -> (usize, usize) {
    if dir.is_empty() {
        return (0, 0);
    }
    match rows.iter().position(|one| one.dir && one.path == dir) {
        Some(at) => (at + 1, rows[at].depth + 1),
        None => (0, 0),
    }
}

/// Every directory of the worktree, with what stands directly in it.
fn tree(paths: &[FileDiff]) -> BTreeMap<String, Node> {
    let mut tree: BTreeMap<String, Node> = BTreeMap::new();
    for file in paths {
        let mut parent = String::new();
        let parts: Vec<&str> = file.path.split('/').collect();
        for (at, part) in parts.iter().enumerate() {
            let last = at + 1 == parts.len();
            let here = match parent.is_empty() {
                true => (*part).to_string(),
                false => format!("{parent}/{part}"),
            };
            let node = tree.entry(parent.clone()).or_default();
            match last {
                true => node.files.insert(here.clone()),
                false => node.dirs.insert(here.clone()),
            };
            parent = here;
        }
    }
    tree
}

/// One directory's own rows, then the rows of every directory open inside it.
fn walk<'a>(
    tree: &BTreeMap<String, Node>,
    dir: &str,
    depth: usize,
    opened: &BTreeSet<String>,
    changed: &[&'a FileDiff],
    out: &mut Vec<Row<'a>>,
) {
    let Some(node) = tree.get(dir) else {
        return;
    };
    for path in &node.dirs {
        out.push(Row {
            depth,
            path: path.clone(),
            name: name_of(path).to_string(),
            dir: true,
            file: None,
        });
        if opened.contains(path) {
            walk(tree, path, depth + 1, opened, changed, out);
        }
    }
    for path in &node.files {
        out.push(Row {
            depth,
            path: path.clone(),
            name: name_of(path).to_string(),
            dir: false,
            file: changed.iter().copied().find(|one| &one.path == path),
        });
    }
}

fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The tree drawn, scrolled and clipped to the list's own room.
pub(super) fn draw(
    ctx: &mut Ctx,
    body: Rect,
    app: &groove_controllers::AppState,
    held: &[Row<'_>],
    open: Option<&String>,
    ui: &Ui,
) {
    let height = ctx.tokens.row;
    let extent = (height * held.len() as f32 - body.h).max(0.0);
    ctx.scrolls(Scroller::Files, extent);
    let scroll = ui.session.files.min(extent);
    ctx.clipped(body, |ctx| {
        let mut y = body.y - scroll;
        for one in held {
            let line = Rect::new(body.x, y, body.w, height);
            if y + height >= body.y && y <= body.bottom() {
                one_row(ctx, line, app, one, open, ui);
            }
            y += height;
        }
    });
}

fn one_row(
    ctx: &mut Ctx,
    line: Rect,
    app: &groove_controllers::AppState,
    held: &Row<'_>,
    open: Option<&String>,
    ui: &Ui,
) {
    let indent = ctx.tokens.md + ctx.tokens.md * held.depth as f32;
    if held.path.is_empty() || renaming(ui, held) {
        return naming(ctx, line, indent, ui);
    }
    if ui.losing() == Some(&crate::Losing::Path(held.path.clone())) {
        return super::asking(ctx, line, "delete it?", ui);
    }
    match (held.dir, held.file) {
        (true, _) => directory(ctx, line, held, indent, ui),
        (false, Some(file)) => {
            let reading = Reading {
                open: open == Some(&file.path),
                noted: super::noted(app, &file.path),
            };
            entry(ctx, line, file, indent, reading, ui)
        }
        (false, None) => plain(ctx, line, held, indent, open, ui),
    }
}

/// Whether this row is the one whose name is being typed over.
fn renaming(ui: &Ui, held: &Row<'_>) -> bool {
    ui.session.naming.as_ref().is_some_and(|naming| {
        matches!(naming.asked, Asked::Rename | Asked::Copy) && naming.at == held.path
    })
}

/// The name being typed, in the row's own place.
pub(super) fn naming(ctx: &mut Ctx, line: Rect, indent: f32, ui: &Ui) {
    let Some(naming) = ui.session.naming.as_ref() else {
        return;
    };
    ctx.quad(line, ctx.styles.raised());
    let style = ctx.styles.code(Role::Text);
    let at = indent + ctx.tokens.small + ctx.tokens.xs;
    let room = (line.w - at - ctx.tokens.md).max(0.0);
    let held = Rect::new(line.x, line.y, room + at, line.h);
    let text = naming.field.shown();
    row(ctx, held, at, &text, style);
}

/// A directory: a twisty, then its own name.
fn directory(ctx: &mut Ctx, line: Rect, held: &Row<'_>, indent: f32, ui: &Ui) {
    let target = Target::Dir(held.path.clone());
    if ui.hover.as_ref() == Some(&target) {
        ctx.quad(line, ctx.styles.hover());
    }
    ctx.hit(line, target);
    let size = ctx.tokens.small;
    let box_ = box_in(line, line.x + indent, size);
    let turn = match ui.session.opened.contains(&held.path) {
        true => 0,
        false => Mark::RIGHTWARDS,
    };
    ctx.icon(box_, Mark::Down, turn, ctx.styles.color(Role::Faint));
    let style = ctx.styles.body(Role::Muted);
    let at = indent + size + ctx.tokens.xs;
    let room = (line.w - at - ctx.tokens.md).max(0.0);
    let text = elide(ctx, &held.name, &style, room);
    row(ctx, line, at, &text, style);
}

/// A file the diff says nothing about: its name, and nothing else.
fn plain(ctx: &mut Ctx, line: Rect, held: &Row<'_>, indent: f32, open: Option<&String>, ui: &Ui) {
    let target = Target::File(held.path.clone());
    if ui.hover.as_ref() == Some(&target) {
        ctx.quad(line, ctx.styles.hover());
    }
    if open == Some(&held.path) {
        crate::widget::ruled(ctx, line, ctx.styles.here());
    }
    ctx.hit(line, target);
    let style = ctx.styles.body(Role::Text);
    let at = indent + ctx.tokens.small + ctx.tokens.xs;
    let room = (line.w - at - ctx.tokens.md).max(0.0);
    let text = elide(ctx, &held.name, &style, room);
    row(ctx, line, at, &text, style);
}
