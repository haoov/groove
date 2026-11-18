//! What a resource's right-click offers: its writes, and the skills that act on its kind, sent with the resource.

use groove_controllers::AppState;

use crate::views::session::resources::Link;
use groove_controllers::cluster_service::restartable;

use crate::{Corner, Losing, Menu, Of, Offer, Ui};

/// What a list row or an Application's row offers: a restart when it rolls out pods, then the skills of its kind.
pub(super) fn menu(
    ui: &Ui,
    app: &AppState,
    target: &crate::hit::Target,
    at: (f32, f32),
) -> Option<Menu> {
    let link = linked(ui, app, target)?;
    let session = app.session.selected()?.session.id.clone();
    let skills = app.skill.skills.iter().filter(|one| one.enabled);
    let offered: Vec<Offer> = skills
        .filter(|one| one.acts_on(&link.kind.kind))
        .map(|one| Offer {
            id: one.id.clone(),
            args: Some(told(&link)),
            label: one.label.clone(),
        })
        .collect();
    let writes: Vec<Losing> = match target {
        crate::hit::Target::ResourceRow(uid) if restartable(&link.kind) => vec![Losing::Restart {
            session: session.clone(),
            uid: uid.clone(),
            link: Box::new(link),
        }],
        _ => Vec::new(),
    };
    if writes.is_empty() && offered.is_empty() {
        return None;
    }
    Some(Menu {
        at,
        corner: Corner::TopLeft,
        of: Of::Resource {
            session,
            writes,
            offered,
        },
    })
}

/// The resource a target stands for.
fn linked(ui: &Ui, app: &AppState, target: &crate::hit::Target) -> Option<Link> {
    match target {
        crate::hit::Target::ResourceRow(uid) => {
            let open = app.session.selected()?;
            let held = &ui.session.resources;
            crate::views::session::resources::row_link(app, open, held, uid)
        }
        crate::hit::Target::ResourceOpen(link) => Some((**link).clone()),
        _ => None,
    }
}

/// `context: prod, namespace: api, kind: Pod (pods), name: api-0`.
fn told(link: &Link) -> String {
    let kind = &link.kind;
    let resource = match kind.group.is_empty() {
        true => kind.plural.clone(),
        false => format!("{}.{}", kind.plural, kind.group),
    };
    let namespace = link.namespace.as_deref().unwrap_or("none, cluster-wide");
    format!(
        "context: {}, namespace: {namespace}, kind: {} ({resource}), name: {}",
        link.context, kind.kind, link.name
    )
}
