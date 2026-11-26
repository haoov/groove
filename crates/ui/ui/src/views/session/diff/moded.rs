//! The Diff and Files tabs in the modes layout: one file, its head on one band over its rows,
//! and a thin map of that file beside them, all on the ground.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::CommitEntry;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in, row_in};
use groove_ui_kit::shape::{Panel, from_right, from_top, hairline, square};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::code::read_box;
use groove_ui_kit::widgets::{Button, cap};

use super::Inline;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;

/// The middle in `area`, a gutter as the conversation's at its sides.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let mut column = area.pad(Edges::xy(ctx.tokens.xl, ctx.tokens.sm));
    if let Some(one) = &app.workspace.commit {
        let band = from_top(&mut column, ctx.tokens.bar, ctx.tokens.sm);
        committed(ctx, band, one);
    }
    body(ctx, app, ui, column);
}

/// The Files tab: the open file's band over its rows, the thin map beside them.
pub fn file(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let column = area.pad(Edges::xy(ctx.tokens.xl, ctx.tokens.sm));
    let tall = |height: f32| Spec::default().height(height);
    let [band, _, body] = column_in(
        column,
        [tall(ctx.tokens.row), tall(ctx.tokens.sm), Spec::fill()],
    );
    let path = app.workspace.active().map(|one| one.path.as_str());
    super::header::written(ctx, band, app, path.unwrap_or_default());
    hairline(ctx, band, ctx.styles.panel_edge());
    match super::whole_file(app, ui) {
        Some(true) => super::said(ctx, body, "Too long to show."),
        Some(false) => super::said(ctx, body, "Open a file in the sidebar."),
        None => mapped(ctx, app, ui, body),
    }
}

/// The rows in `body` with the thin map at their right.
fn mapped(ctx: &mut Ctx, app: &AppState, ui: &Ui, body: Rect) {
    let wide = |width: f32| Spec::default().width(width);
    let [body, _, map] = row_in(
        body,
        [Spec::fill(), wide(ctx.tokens.sm), wide(ctx.tokens.map_thin)],
    );
    let cols = super::surface::note_cols(ctx, body, app, ui);
    ctx.app.hits.wraps(cols);
    let inline = Inline::of(app, ui, ui.session.face(), cols);
    super::map::draw(ctx, (map, body), app, ui, &inline);
    super::rows(ctx, body, app, ui, &inline);
}

/// `COMMIT 3f2a91c  fix(ui): light tints · Alex`, and what puts the working tree back.
fn committed(ctx: &mut Ctx, band: Rect, one: &CommitEntry) {
    let panel = Panel::default().ground(ctx.styles.band_tint(Role::Accent));
    panel
        .border(ctx.styles.band_edge(Role::Accent))
        .draw(ctx, band);
    let sm = ctx.tokens.sm;
    let mut room = band.pad(Edges::across(ctx.tokens.md, sm));
    let back = Button::glass(
        "back to working tree",
        Target::Working,
        Role::Text,
        &ctx.styles,
    );
    back.right(ctx, &mut room, sm);
    cap(ctx, &mut room, "commit", Role::Accent);
    Label::new(&one.short_sha, ctx.styles.code(Role::Text)).left(ctx, &mut room, ctx.tokens.md);
    let subject = one.message.lines().next().unwrap_or_default();
    let said = format!("{subject} · {}", one.author);
    Label::new(&said, ctx.styles.body(Role::Muted)).draw(ctx, room);
}

/// The band and the file's rows in `column`, which the cluster write's band may stand over.
pub fn body(ctx: &mut Ctx, app: &AppState, ui: &Ui, column: Rect) {
    let tall = |height: f32| Spec::default().height(height);
    let [band, _, body] = column_in(
        column,
        [tall(ctx.tokens.row), tall(ctx.tokens.sm), Spec::fill()],
    );
    headed(ctx, app, ui, band);
    hairline(ctx, band, ctx.styles.panel_edge());
    if app.workspace.changes.is_empty() {
        return super::hint(ctx, app, body);
    }
    mapped(ctx, app, ui, body);
}

/// The file's name, folder and counts at the left; the switches and its read box at the right.
fn headed(ctx: &mut Ctx, app: &AppState, ui: &Ui, band: Rect) {
    let mut room = band;
    let path = app.workspace.changes.focused().map(str::to_string);
    if let Some(path) = &path {
        let size = ctx.tokens.icon;
        let mark = square(from_right(&mut room, size, ctx.tokens.md), size);
        read_box(ctx, mark, super::row::is_read(app, path));
        ctx.hit(mark, Target::Read(path.clone()));
    }
    super::header::switches(ctx, &mut room, app, ui);
    if let Some(path) = &path {
        named(ctx, app, &mut room, path);
    }
}

/// `alpha.rs  src/a  +2 -1`, each cut to what is left of `room`.
fn named(ctx: &mut Ctx, app: &AppState, room: &mut Rect, path: &str) {
    let sm = ctx.tokens.sm;
    let read = super::row::is_read(app, path);
    let role = if read { Role::Faint } else { Role::Text };
    let (folder, name) = path.rsplit_once('/').unwrap_or(("", path));
    Label::new(name, ctx.styles.file_head(role)).left(ctx, room, sm);
    if !folder.is_empty() {
        Label::new(folder, ctx.styles.body(Role::Faint)).left(ctx, room, sm);
    }
    let (added, deleted) = super::row::counted(app, path).unwrap_or_default();
    for (count, role, sign) in [(added, Role::Ok, '+'), (deleted, Role::Bad, '-')] {
        if count > 0 {
            let text = format!("{sign}{count}");
            Label::new(&text, ctx.styles.code(role)).left(ctx, room, sm);
        }
    }
}
