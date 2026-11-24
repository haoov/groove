//! A session carried across the rail: where it would land, and how the carry is drawn.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::{Section, SessionId};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::Tokens;
use groove_ui_kit::shape::{band, ground};
use groove_ui_kit::widgets::{glass_lit, glass_pane};

use super::item;
use super::order::{Entry, entries};
use crate::ctx::Ctx;
use crate::{Held, Ui};

/// Where a carried session would land: the gap before the `index`th row of `section`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub section: Section,
    pub index: usize,
}

/// The session the rail carries, and where its lifted row's top stands.
pub(crate) fn carried(ui: &Ui) -> Option<(&SessionId, f32)> {
    match &ui.held {
        Some(Held::Session { id, at, grip, .. }) => Some((id, at - grip)),
        _ => None,
    }
}

/// Where `id`'s row stands now, its top in the window.
pub(crate) fn top_of(tokens: &Tokens, app: &AppState, ui: &Ui, rows: Rect, id: &SessionId) -> Option<f32> {
    let mut y = rows.y - scroll(tokens, app, ui, rows);
    for entry in entries(app, false, ui.rail.shut) {
        if matches!(&entry, Entry::Session(open) if &open.session.id == id) {
            return Some(y);
        }
        y += height_of(tokens, &entry);
    }
    None
}

/// How tall a line of the rail stands.
pub(crate) fn height_of(tokens: &Tokens, entry: &Entry) -> f32 {
    match entry {
        Entry::Heading(..) => tokens.row,
        Entry::Session(_) => item::height(tokens),
        Entry::Child { .. } => item::child_height(tokens),
    }
}

/// Every gap a drop can take, with how far down the list it stands.
fn gaps(tokens: &Tokens, app: &AppState, ui: &Ui) -> Vec<(f32, Slot)> {
    let lines = entries(app, true, ui.rail.shut);
    let (mut y, mut out, mut count) = (0.0, Vec::new(), 0);
    let mut section = Section::Active;
    for (at, entry) in lines.iter().enumerate() {
        y += height_of(tokens, entry);
        match entry {
            Entry::Heading(one, _) => {
                (section, count) = (*one, 0);
                out.push((y, Slot { section, index: 0 }));
                continue;
            }
            Entry::Session(_) => count += 1,
            Entry::Child { .. } => {}
        }
        let next_is_child = matches!(lines.get(at + 1), Some(Entry::Child { .. }));
        if !next_is_child {
            let index = count;
            out.push((y, Slot { section, index }));
        }
    }
    out
}

/// The gap nearest to `y`, measured in `rows`, the list's room.
pub(crate) fn slot_at(tokens: &Tokens, app: &AppState, ui: &Ui, rows: Rect, y: f32) -> Option<Slot> {
    let at = y - rows.y + scroll(tokens, app, ui, rows);
    let gaps = gaps(tokens, app, ui);
    let near = |one: &&(f32, Slot)| (one.0 - at).abs();
    let nearest = gaps.iter().min_by(|a, b| near(a).total_cmp(&near(b)));
    nearest.map(|one| one.1)
}

/// What the drop asks: the session, its section, its index once taken out of its own place.
/// Nothing while it would not move.
pub(crate) fn landing(app: &AppState, ui: &Ui) -> Option<(SessionId, Section, usize)> {
    let (id, _) = carried(ui)?;
    let slot = ui.rail.drop?;
    let open = app.session.get(id)?;
    let tops = app.session.tops(open.state.section);
    let own = tops.iter().position(|one| &one.session.id == id);
    let mut index = slot.index;
    if open.state.section == slot.section
        && let Some(own) = own
    {
        if own < index {
            index -= 1;
        }
        if own == index {
            return None;
        }
    }
    Some((id.clone(), slot.section, index))
}

/// How far the list stands scrolled, as the list itself clamps it.
fn scroll(tokens: &Tokens, app: &AppState, ui: &Ui, rows: Rect) -> f32 {
    let lines = entries(app, true, ui.rail.shut);
    let content: f32 = lines.iter().map(|one| height_of(tokens, one)).sum();
    ui.rail.scroll.min((content - rows.h).max(0.0))
}

/// The section under the pointer lit, a rule where the row would land, the row lifted on glass.
pub(crate) fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, rows: Rect) {
    let (Some((id, at)), Some(slot)) = (carried(ui), ui.rail.drop) else {
        return;
    };
    let tokens = ctx.tokens;
    let scrolled = scroll(&tokens, app, ui, rows);
    let mut y = -scrolled;
    for entry in entries(app, true, ui.rail.shut) {
        let tall = height_of(&tokens, &entry);
        if matches!(entry, Entry::Heading(one, _) if one == slot.section) {
            glass_lit(ctx, band(rows, y, tall));
        }
        y += tall;
    }
    let moves = landing(app, ui).is_some();
    let gaps = gaps(&tokens, app, ui);
    if let Some((edge, _)) = gaps.iter().find(|one| moves && one.1 == slot) {
        let thick = tokens.hairline * 2.0;
        let top = edge - scrolled - thick / 2.0;
        ground(ctx, band(rows, top, thick), Ground::Solid(Role::Working));
    }
    let Some(open) = app.session.get(id) else {
        return;
    };
    let tall = item::height(&tokens);
    let top = (at - rows.y).clamp(0.0, (rows.h - tall).max(0.0));
    let lifted = band(rows, top, tall);
    ctx.layer();
    glass_pane(ctx, lifted);
    item::draw(ctx, app, ui, lifted, open);
}
