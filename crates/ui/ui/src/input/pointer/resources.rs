//! What a click on the Resources tab does: a scope picker opens its panel, a kind is listed.

use groove_controllers::{AppState, Command};

use crate::hit::{Hits, Picks, Target};
use crate::views::session::resources::{self, Dragged, How, ResourcesUi, Scoping};
use crate::{Held, Overlay, Ui};
use groove_ui_kit::base::ctx::Metrics;

pub(super) fn acted(target: &Target, ui: &mut Ui, app: &AppState) -> Option<Vec<Command>> {
    let held = &mut ui.session.resources;
    match target {
        Target::Picker(which @ (Picks::Contexts | Picks::Namespaces)) => {
            let open = app.session.selected()?;
            let (scoping, commands) = Scoping::new(*which).opened(open, held);
            ui.overlay = Some(Overlay::Scope(scoping));
            return Some(commands);
        }
        Target::ResourceKind(kind) => {
            (held.kind, held.scroll, held.showing) = (Some(kind.clone()), 0.0, None);
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
        Target::ResourceCopy(text, _) => {
            if let Some(tab) = held.tab_mut() {
                tab.copied = Some(text.clone());
            }
            let copy = groove_controllers::cluster::Command::CopyValue { text: text.clone() };
            return Some(vec![Command::Cluster(copy)]);
        }
        Target::ResourceList => held.showing = None,
        Target::ResourceRow(uid) => {
            let open = app.session.selected()?;
            let link = resources::row_link(app, open, &ui.session.resources, uid)?;
            ui.session.resources.open(link);
        }
        _ if tabbed(target, held) => {}
        _ => return None,
    }
    Some(Vec::new())
}

/// A ctrl+click on a link: its object's tab, not a copy.
pub(super) fn followed(ui: &mut Ui, link: resources::Link) -> Vec<Command> {
    ui.session.resources.open(link);
    Vec::new()
}

/// A click on an object's tab, in the strip or inside it. False for any other target.
fn tabbed(target: &Target, held: &mut ResourcesUi) -> bool {
    match target {
        Target::ResourceOpen(link) => held.open((**link).clone()),
        Target::ResourceTab(at) => held.showing = Some(*at),
        Target::ResourceClose(at) => held.close(*at),
        _ => {
            let Some(tab) = held.tab_mut() else {
                return false;
            };
            match target {
                Target::ResourceView(yaml) => (tab.yaml, tab.scroll) = (*yaml, 0.0),
                Target::ResourceContainer(name) => tab.container = Some(name.clone()),
                Target::ResourceSection(section) => {
                    if !tab.shut.remove(section) {
                        tab.shut.insert(*section);
                    }
                }
                _ => return false,
            }
        }
    }
    true
}

/// A click while a scope panel is open: a line chosen, its × detaching; outside, the panel shuts.
pub(super) fn scoped(target: Option<Target>, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let how = match target {
        Some(Target::ScopeLine(at)) => (at, How::Toggle),
        Some(Target::ScopeDetach(at)) => (at, How::Detach),
        Some(Target::ScopePanel) => return Vec::new(),
        _ => {
            ui.overlay = None;
            return Vec::new();
        }
    };
    super::super::keys::pick(how, ui, app)
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
