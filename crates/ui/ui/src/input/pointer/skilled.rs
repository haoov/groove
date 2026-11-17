//! What a resource's right-click offers: its writes, and the skills that act on its kind, sent with the resource.

use groove_controllers::AppState;

use groove_controllers::cluster_service::{restartable, scalable};
use groove_types::SessionId;

use crate::views::session::resources::{Link, Write, Written, desired};

use crate::{Corner, Losing, Menu, Of, Offer, Shelled, Ui};

/// What a list row or an Application's row offers: a list row's writes, then the skills of its kind.
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
    let (shells, writes) = match target {
        crate::hit::Target::ResourceRow(uid) => {
            (shells(app, &link), writes(ui, app, (&session, uid), &link))
        }
        _ => Default::default(),
    };
    if writes.is_empty() && offered.is_empty() {
        return None;
    }
    Some(Menu {
        at,
        corner: Corner::TopLeft,
        of: Of::Resource {
            session,
            shells,
            writes,
            offered,
        },
    })
}

/// What a list row asks before it writes: a scale and a restart for a workload, then a delete.
fn writes(ui: &Ui, app: &AppState, (session, uid): (&SessionId, &str), link: &Link) -> Vec<Losing> {
    let mut writes = Vec::new();
    if scalable(&link.kind) {
        let open = app.session.selected();
        let held = &ui.session.resources;
        let now = open.and_then(|open| desired(app, open, held, uid));
        writes.push(Write::Scale(now.unwrap_or(1)));
    }
    if restartable(&link.kind) {
        writes.push(Write::Restart);
    }
    writes.push(Write::Delete);
    let asked = |write| {
        Losing::Row(Box::new(Written {
            session: session.clone(),
            uid: uid.to_string(),
            link: link.clone(),
            write,
        }))
    };
    writes.into_iter().map(asked).collect()
}

/// A pod's shell: one per container once its tab has read them and it has several.
fn shells(app: &AppState, link: &Link) -> Vec<Shelled> {
    let kind = &link.kind;
    if kind.kind != "Pod" || !kind.group.is_empty() {
        return Vec::new();
    }
    let pod = Box::new(groove_types::Target {
        context: link.context.clone(),
        kind: kind.clone(),
        namespace: link.namespace.clone(),
        name: link.name.clone(),
        subresource: None,
    });
    let read = link.read(app).and_then(|one| one.pod.clone());
    let containers = read.map(|one| one.containers).unwrap_or_default();
    let names: Vec<String> = containers
        .into_iter()
        .filter(|one| !one.init)
        .map(|one| one.name)
        .collect();
    let shelled = |container: Option<String>| Shelled {
        pod: pod.clone(),
        label: container
            .as_ref()
            .map_or("shell".into(), |name| format!("shell · {name}")),
        container,
    };
    match names.len() < 2 {
        true => vec![shelled(None)],
        false => names.into_iter().map(Some).map(shelled).collect(),
    }
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
        crate::hit::Target::ResourceDrift(_, link) => link.as_deref().cloned(),
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
