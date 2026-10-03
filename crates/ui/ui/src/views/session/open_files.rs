//! The Files tab's strip: one tab per open file, a dot while it owes the disk, a cross to close it.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Opened;
use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::{Losing, Ui};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Tab, Text};

pub fn draw(ctx: &mut Ctx, strip: Rect, app: &AppState, ui: &Ui) {
    groove_ui_kit::shape::ground(ctx, strip, Ground::Band);
    hairline(ctx, strip, ctx.styles.line());
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
    let mut room = strip;
    for one in open {
        let preview = buffers.is_some_and(|all| all.previews(&one.path));
        tab(
            ctx,
            &mut room,
            one,
            (active == Some(one.path.as_str()), preview),
        );
    }
}

/// One file's tab from the left of `room`, its name in italic while it is the preview.
fn tab(ctx: &mut Ctx, room: &mut Rect, open: &Opened, (active, preview): (bool, bool)) {
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
    let ground = ctx.styles.ground();
    tab.text(Text::Small)
        .italic(preview)
        .quiet(Role::Muted)
        .ground(ground)
        .close(mark, close)
        .left(ctx, room, 0.0);
    let rule = room.take_left(ctx.tokens.hairline);
    groove_ui_kit::shape::side_rule(ctx, rule, rule.x, ctx.styles.line());
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
