//! What a menu offers, and where it is drawn.

use groove_ui_kit::base::style::Role;

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::{Corner, Menu, Of};

mod act;
mod picked;
pub use act::Act;
use groove_ui_kit::widgets::{menu, menu_size};
pub use picked::picked;

/// The actions of one file.
pub const FILE: [Act; 1] = [Act::DiscardChanges];

/// The actions of one open file's tab.
pub const TAB: [Act; 3] = [Act::Close, Act::CloseOthers, Act::CloseAll];

/// What the lines under a click offer.
pub const LINE: [Act; 1] = [Act::Note];

/// The actions of the worktree, from the commit box.
pub const WORKTREE: [Act; 3] = [Act::Push, Act::Pull, Act::DiscardEverything];

/// The same, for a worktree whose branch already has a merge request.
pub const WORKTREE_MR: [Act; 6] = [
    Act::Push,
    Act::Pull,
    Act::UpdateMr,
    Act::Comment,
    Act::CloseMr,
    Act::DiscardEverything,
];

/// What a session reviewing someone else's merge request offers.
pub const WORKTREE_REVIEW: [Act; 6] = [
    Act::Pull,
    Act::Comment,
    Act::Approve,
    Act::RequestChanges,
    Act::CloseMr,
    Act::DiscardEverything,
];

/// The actions of one path of the explorer.
pub const PATH: [Act; 5] = [
    Act::NewFile,
    Act::NewDirectory,
    Act::Rename,
    Act::Copy,
    Act::Delete,
];

/// The actions of the session, from the header.
pub const SESSION: [Act; 1] = [Act::DeleteLocally];

/// The actions a menu of fixed rows offers; none for a menu whose rows come with it.
fn acts(of: &Of) -> &'static [Act] {
    match of {
        Of::File(_) => &FILE,
        Of::Tab { .. } => &TAB,
        Of::Line { .. } => &LINE,
        Of::Path { .. } => &PATH,
        Of::Worktree { review: true, .. } => &WORKTREE_REVIEW,
        Of::Worktree { mr: true, .. } => &WORKTREE_MR,
        Of::Worktree { mr: false, .. } => &WORKTREE,
        Of::Session(_) => &SESSION,
        Of::Skills { .. } | Of::Mapping(_) | Of::Hues(_) | Of::LogRanges => &[],
    }
}

pub fn rows(of: &Of) -> Vec<&str> {
    match of {
        Of::Skills { offered, .. } => offered.iter().map(|one| one.label.as_str()).collect(),
        Of::Mapping(choices) if choices.options.is_empty() => vec![choices.empty],
        Of::Mapping(choices) => choices.options.iter().map(String::as_str).collect(),
        Of::Hues(_) => groove_types::Hue::ALL
            .iter()
            .map(|one| one.name())
            .collect(),
        Of::LogRanges => crate::views::session::resources::RANGES.to_vec(),
        _ => acts(of).iter().map(|one| one.label()).collect(),
    }
}

pub fn draw(ctx: &mut Ctx, open: &Menu) {
    let within = ctx.window;
    let rows = rows(&open.of);
    let (wide, tall) = menu_size(ctx, &rows);
    let at = match open.corner {
        Corner::TopLeft => open.at,
        Corner::BottomLeft => (open.at.0, open.at.1 - tall),
        Corner::BottomRight => (open.at.0 - wide, open.at.1 - tall),
    };
    ctx.layer();
    let edge = ctx.styles.border();
    let roles: Vec<Role> = match &open.of {
        Of::Hues(_) => groove_types::Hue::ALL
            .iter()
            .map(|one| Role::Hue(*one))
            .collect(),
        _ => Vec::new(),
    };
    menu(ctx, at, within, (&rows, &roles), edge, Target::MenuRow);
}
