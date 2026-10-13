//! The skills a resource's right-click offers: those that act on its kind, sent with the resource.

use groove_controllers::AppState;

use crate::views::session::resources::Link;
use crate::{Corner, Menu, Of, Offer, Ui};

/// The menu of skills for the resource a list row or an object tab stands for.
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
    if offered.is_empty() {
        return None;
    }
    Some(Menu {
        at,
        corner: Corner::TopLeft,
        of: Of::Skills { session, offered },
    })
}

/// The resource a target stands for.
fn linked(ui: &Ui, app: &AppState, target: &crate::hit::Target) -> Option<Link> {
    let held = &ui.session.resources;
    match target {
        crate::hit::Target::ResourceRow(uid) => {
            let open = app.session.selected()?;
            crate::views::session::resources::row_link(app, open, held, uid)
        }
        crate::hit::Target::ResourceTab(at) => held.opened.get(*at).map(|one| one.link.clone()),
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
