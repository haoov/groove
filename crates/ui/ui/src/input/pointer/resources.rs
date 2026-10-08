//! What a click on the Resources tab does: a scope picker opens its menu, a kind is listed.

use groove_controllers::{AppState, Command};

use crate::hit::{Hits, Picks, Target};
use crate::views::session::resources::Dragged;
use crate::views::session::resources::{self, Pick};
use crate::{Corner, Held, Menu, Of, Overlay, Ui};
use groove_ui_kit::base::ctx::Metrics;

pub(super) fn acted(
    target: &Target,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
) -> Option<Vec<Command>> {
    let held = &mut ui.session.resources;
    match target {
        Target::Picker(which @ (Picks::Contexts | Picks::Namespaces)) => {
            let under = hits.rect_of(target)?;
            let picks = picks(app, ui, *which)?;
            ui.overlay = Some(Overlay::Menu(Menu {
                at: (under.x, under.bottom()),
                corner: Corner::TopLeft,
                of: Of::Scope(picks),
            }));
        }
        Target::ResourceKind(kind) => {
            held.kind = Some(kind.clone());
            held.scroll = 0.0;
        }
        Target::ResourceSearch => (held.typing, held.filtering) = (true, false),
        Target::ResourceFilter => (held.filtering, held.typing) = (true, false),
        Target::ResourceGroup(group) => {
            if !held.folded.remove(group) {
                held.folded.insert(group.clone());
            }
        }
        Target::ResourceSort(label) => {
            held.sort = match held.sort.take() {
                Some((at, descending)) if at == *label => Some((at, !descending)),
                _ => Some((label.clone(), false)),
            };
        }
        Target::ResourceList | Target::ResourceRow(_) => {}
        _ => return None,
    }
    Some(Vec::new())
}

/// What a picker offers, each with its label: the contexts, or the pairs of the ones picked.
fn picks(app: &AppState, ui: &Ui, which: Picks) -> Option<Vec<(Pick, String)>> {
    let open = app.session.selected()?;
    let picks: Vec<Pick> = match which {
        Picks::Contexts => resources::contexts(open)
            .into_iter()
            .map(|one| Pick::Context(one.into()))
            .collect(),
        _ => resources::offered(open, &ui.session.resources)
            .cloned()
            .map(Pick::Pair)
            .collect(),
    };
    let labelled = |pick: Pick| {
        let label = pick.label();
        (pick, label)
    };
    Some(picks.into_iter().map(labelled).collect())
}

/// A press on a column's edge holds it, at the width its header was drawn.
pub(super) fn held(
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    (label, x): (String, f32),
) -> Vec<Command> {
    let open = app.session.selected();
    let kind = open.and_then(|open| resources::kind(app, open, &ui.session.resources));
    let header = hits.rect_of(&Target::ResourceSort(label.clone()));
    if let (Some(kind), Some(header)) = (kind, header) {
        let width = header.w;
        ui.held = Some(Held::Column(Dragged {
            kind: kind.plural,
            label,
            from: x,
            width,
        }));
    }
    Vec::new()
}

/// The held column follows the pointer, never narrower than a mark.
pub(super) fn dragged(ui: &mut Ui, x: f32, metrics: Metrics) {
    let Some(Held::Column(drag)) = ui.held.clone() else {
        return;
    };
    let least = metrics.tokens().icon * 2.0;
    let width = (drag.width + x - drag.from).max(least);
    let widths = ui.session.resources.widths.entry(drag.kind).or_default();
    widths.insert(drag.label, width);
}
