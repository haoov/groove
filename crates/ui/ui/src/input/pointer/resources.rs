//! What a click in the Kubernetes mode does: a kind is listed, an object opened.

use groove_controllers::{AppState, Command, cluster, session, workspace};

use crate::hit::{Hits, Target};
use crate::views::session::Mode;
use crate::views::session::resources::{self, Columns, Dragged, How, Opens, ResourcesUi};
use crate::{Corner, Menu, Of};
use crate::{Held, Overlay, Ui};
use groove_ui_kit::base::ctx::Metrics;

pub(super) fn acted(
    target: &Target,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
) -> Option<Vec<Command>> {
    logs(target, ui, hits)
        .or_else(|| pinned(target, ui, app))
        .or_else(|| drafted(target, ui, app))
        .or_else(|| listed(target, ui, app))
}

/// The band's pin toggled, a pinned list shown, or let go by its cross.
fn pinned(target: &Target, ui: &mut Ui, app: &AppState) -> Option<Vec<Command>> {
    let open = app.session.selected()?;
    let held = &mut ui.session.resources;
    match target {
        Target::ResourcePin => Some(resources::pins::toggled(app, held)),
        Target::PinnedList(pin) => {
            resources::pins::restore(held, app, open, pin);
            Some(Vec::new())
        }
        Target::Unpin(pin) => {
            let session = open.session.id.clone();
            let pin = (**pin).clone();
            Some(vec![Command::Session(session::Command::Unpin {
                session,
                pin,
            })])
        }
        _ => None,
    }
}

/// A click on the YAML's bar: EDIT, APPLY or DISCARD.
fn drafted(target: &Target, ui: &Ui, app: &AppState) -> Option<Vec<Command>> {
    let key = Box::new(ui.session.resources.tab()?.link.key());
    let asked = match target {
        Target::YamlWrite(on) => {
            let (session, on) = (app.session.selected.clone()?, *on);
            cluster::Command::WriteYaml { session, key, on }
        }
        Target::YamlApply => {
            let session = app.session.selected.clone()?;
            cluster::Command::ApplyYaml { session, key }
        }
        _ => return None,
    };
    Some(vec![Command::Cluster(asked)])
}

/// A click on the sidebar, the list, or an object's tab.
fn listed(target: &Target, ui: &mut Ui, app: &AppState) -> Option<Vec<Command>> {
    let held = &mut ui.session.resources;
    match target {
        Target::ResourceKind(kind) => {
            (held.kind, held.scroll, held.showing) = (Some(kind.clone()), 0.0, None);
        }
        Target::ResourceSearch => held.typing = true,
        Target::ResourceGroup(group) => {
            groove_types::toggle(&mut held.folded, group.clone());
        }
        Target::ResourceSort(label) => {
            let sort = match held.tab_mut() {
                Some(tab) => &mut tab.sort,
                None => &mut held.sort,
            };
            *sort = match sort.take() {
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

/// A ctrl+click on a link, not a copy: its object's tab, its repo in the session, or its commit's diff.
pub(super) fn followed(ui: &mut Ui, opens: Opens) -> Vec<Command> {
    let shown = |repo, target, sha| {
        let target = Some(target);
        let show = workspace::Command::ShowRepo { repo, target, sha };
        vec![Command::Workspace(show)]
    };
    match opens {
        Opens::Object(link) => {
            ui.session.resources.open(link);
            Vec::new()
        }
        Opens::Repo { repo, target } => {
            ui.session.going = Some((Mode::Files, None));
            shown(repo, target, None)
        }
        Opens::Revision { repo, target, sha } => {
            ui.session.going = Some((Mode::Diff, Some(sha.clone())));
            shown(repo, target, Some(sha))
        }
    }
}

/// A click on the logs view's range picker.
fn logs(target: &Target, ui: &mut Ui, hits: &Hits) -> Option<Vec<Command>> {
    match target {
        Target::LogRange => {
            let under = hits.rect_of(target)?;
            ui.overlay = Some(Overlay::Menu(Menu {
                at: (under.x, under.bottom()),
                corner: Corner::TopLeft,
                of: Of::LogRanges,
            }));
            Some(Vec::new())
        }
        _ => None,
    }
}

/// A range picked for the logs shown; they open again from there.
pub(super) fn ranged(ui: &mut Ui, at: usize) {
    let picked = groove_types::LogRange::OFFERED.get(at);
    if let (Some(tab), Some(range)) = (ui.session.resources.tab_mut(), picked) {
        (tab.logs.range, tab.logs.following) = (*range, true);
    }
}

/// A click on an open object's row, or inside the object. False for any other target.
fn tabbed(target: &Target, held: &mut ResourcesUi) -> bool {
    match target {
        Target::ResourceOpen(link) => held.open((**link).clone()),
        Target::ResourceTab(at) => (held.showing, held.typing) = (Some(*at), false),
        Target::ResourceClose(at) => held.close(*at),
        _ => {
            let Some(tab) = held.tab_mut() else {
                return false;
            };
            match target {
                Target::ResourceView(view) => (tab.view, tab.scroll) = (*view, 0.0),
                Target::LogSource(source) => tab.logs.source = Some(source.clone()),
                Target::LogPrevious(previous) => tab.logs.previous = *previous,
                Target::LogFollow => tab.logs.following = true,
                Target::ArgoSearch => tab.argo.typing = true,
                Target::ArgoFilter(filter) => {
                    (tab.argo.filter, tab.scroll) = (Some(*filter), 0.0);
                    tab.toggled.clear();
                }
                Target::ResourceFold(key, shut) => tab.fold(key, *shut),
                Target::ResourceSection(section) => {
                    groove_types::toggle(&mut tab.shut, *section);
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
    let held = &ui.session.resources;
    let table = match held.tab() {
        Some(_) => Some(Columns::Tab),
        None => open
            .and_then(|open| resources::kind(app, open, held).map(|one| Columns::Kind(one.plural))),
    };
    let header = hits.rect_of(&Target::ResourceSort(label.clone()));
    if let (Some(table), Some(header)) = (table, header) {
        hold(ui, table, label, (x, header.w));
    }
    Vec::new()
}

/// The column's edge held, at the width its header was drawn.
pub(super) fn hold(ui: &mut Ui, table: Columns, label: String, (from, width): (f32, f32)) {
    ui.held = Some(Held::Column(Dragged {
        table,
        label,
        from,
        width,
    }));
}

/// The held column follows the pointer, never narrower than a mark.
pub(super) fn dragged(ui: &mut Ui, x: f32, metrics: Metrics) {
    let Some(Held::Column(drag)) = ui.held.clone() else {
        return;
    };
    let least = metrics.tokens().icon * 2.0;
    let width = (drag.width + x - drag.from).max(least);
    let held = &mut ui.session.resources;
    let widths = match drag.table {
        Columns::Kind(kind) => held.widths.entry(kind).or_default(),
        Columns::Tab => &mut held.tab_widths,
    };
    widths.insert(drag.label, width);
}
