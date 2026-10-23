//! The Files tab's strip: one tab per open file, a dot while it owes the disk, a cross to close it.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Opened;
use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::{Losing, Ui};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::Tab;

pub fn draw(ctx: &mut Ctx, strip: Rect, app: &AppState, ui: &Ui) {
    let open = app
        .workspace
        .buffers()
        .map(|one| one.all())
        .unwrap_or_default();
    if let Some(path) = losing(ui, open) {
        let question = format!("close {} and lose its edits?", crate::views::name_of(path));
        return crate::views::session::files::asking(ctx, strip, &question);
    }
    if open.is_empty() {
        let room = strip.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        let said = "no file open: pick one in the explorer";
        Label::new(said, ctx.styles.small(Role::Faint)).draw(ctx, room);
        return;
    }
    let active = app.workspace.active().map(|one| one.path.as_str());
    let buffers = app.workspace.buffers();
    let mut room = strip.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm));
    for one in open {
        let preview = buffers.is_some_and(|all| all.previews(&one.path));
        let tally = crate::views::session::diff::faults::tally(app, &one.path);
        let state = (active == Some(one.path.as_str()), preview, tally);
        tab(ctx, &mut room, one, state);
    }
}

/// Whether a tab is the active one and the preview, and its file's errors and warnings.
type Shown = (bool, bool, Option<(u32, u32)>);

/// One file's tab from the left of `room`, its name in italic while it is the preview.
fn tab(ctx: &mut Ctx, room: &mut Rect, open: &Opened, (active, preview, tally): Shown) {
    let close = Target::CloseTab(open.path.clone());
    let mark = match open.new.dirty() && !ctx.hovered(&close) {
        true => Mark::Modified,
        false => Mark::Close,
    };
    let tab = Tab::new(
        crate::views::name_of(&open.path),
        Target::OpenTab(open.path.clone()),
        active,
    );
    tab.glass()
        .italic(preview)
        .tally(tally)
        .close(mark, close)
        .left(ctx, room, ctx.tokens.xs);
}

/// The open file whose closing waits on an answer.
fn losing<'a>(ui: &Ui, open: &'a [Opened]) -> Option<&'a str> {
    let Some(Losing::Tab(path)) = ui.losing() else {
        return None;
    };
    open.iter()
        .map(|one| one.path.as_str())
        .find(|one| one == path)
}
