//! What a menu offers, and where it is drawn.

use groove_ui_kit::base::style::Role;

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::{Corner, Menu, Of};

mod act;
mod picked;
pub use act::Act;
use groove_ui_kit::widgets::{glass_menu, glass_size};
pub use picked::picked;

/// The actions of one open file's tab.
pub const TAB: [Act; 3] = [Act::Close, Act::CloseOthers, Act::CloseAll];

/// What the lines under a click offer.
pub const LINE: [Act; 1] = [Act::Ask];

/// The actions of one path of the explorer.
pub const PATH: [Act; 5] = [
    Act::NewFile,
    Act::NewDirectory,
    Act::Rename,
    Act::Copy,
    Act::Delete,
];

/// The actions of a session's row, by the section it stands in; a child only closes.
pub const ACTIVE: [Act; 3] = [Act::Pause, Act::MoveToTop, Act::Close];
pub const PAUSED: [Act; 3] = [Act::Resume, Act::MoveToTop, Act::Close];
pub const CHILD: [Act; 1] = [Act::Close];

/// The actions a menu of fixed rows offers; none for a menu whose rows come with it.
fn acts(of: &Of) -> &'static [Act] {
    match of {
        Of::Tab { .. } | Of::Object(_) => &TAB,
        Of::Line { .. } => &LINE,
        Of::Path { .. } => &PATH,
        Of::Session { section, .. } => match section {
            Some(groove_types::Section::Active) => &ACTIVE,
            Some(groove_types::Section::Paused) => &PAUSED,
            None => &CHILD,
        },
        Of::Resource { .. }
        | Of::Mapping(_)
        | Of::Hues(_)
        | Of::LogRanges
        | Of::Listing(_)
        | Of::Models { .. } => &[],
    }
}

pub fn rows(of: &Of) -> Vec<&str> {
    match of {
        Of::Resource {
            shells,
            writes,
            offered,
            ..
        } => {
            let mut rows: Vec<&str> = shells.iter().map(|one| one.label.as_str()).collect();
            rows.extend(writes.iter().map(|one| one.act()));
            rows.extend(offered.iter().map(|one| one.label.as_str()));
            rows
        }
        Of::Mapping(choices) if choices.options.is_empty() => vec![choices.empty],
        Of::Mapping(choices) => choices.options.iter().map(String::as_str).collect(),
        Of::Hues(_) => groove_types::Hue::ALL
            .iter()
            .map(|one| one.name())
            .collect(),
        Of::LogRanges => crate::views::session::resources::RANGES.to_vec(),
        Of::Listing(true) => vec!["Show as a tree"],
        Of::Listing(false) => vec!["Show as a list"],
        Of::Models { rows, .. } => rows.iter().map(|one| one.label.as_str()).collect(),
        _ => acts(of).iter().map(|one| one.label()).collect(),
    }
}

pub fn draw(ctx: &mut Ctx, open: &Menu) {
    if let Of::Models { rows, .. } = &open.of {
        return crate::views::session::agent_pane::model::menu(ctx, open, rows);
    }
    let within = ctx.window;
    let rows = rows(&open.of);
    let (wide, tall) = glass_size(ctx, &rows);
    let at = match open.corner {
        Corner::TopLeft => open.at,
        Corner::TopRight => (open.at.0 - wide, open.at.1),
        Corner::BottomLeft => (open.at.0, open.at.1 - tall),
        Corner::BottomRight => (open.at.0 - wide, open.at.1 - tall),
    };
    let roles: Vec<Role> = match &open.of {
        Of::Hues(_) => groove_types::Hue::ALL
            .iter()
            .map(|one| Role::Hue(*one))
            .collect(),
        _ => Vec::new(),
    };
    ctx.layer();
    glass_menu(ctx, at, within, (&rows, &roles), Target::MenuRow);
}
