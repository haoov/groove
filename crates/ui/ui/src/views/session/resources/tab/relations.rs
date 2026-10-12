//! What the object stands among, as the list's table: what owns it, what it uses, what uses it.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::Described;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::widgets::{Cell, Shown, Width};

use super::super::around::{lineage, selecting, served};
use super::super::opened::{Link, Opened};
use super::{copy, grouped};
use crate::ctx::Ctx;

/// One object it stands among: a tag, a name, what it says, the tab it opens; set in by `depth`.
pub(super) struct Entry {
    depth: usize,
    tag: String,
    name: String,
    status: Option<(String, Role)>,
    open: Option<Link>,
}

/// A line of the table: a group's heading, which folds, or one of its objects.
pub(super) enum Line {
    Heading {
        title: &'static str,
        count: usize,
        shut: bool,
    },
    Entry(Box<Entry>),
}

/// How many objects the lines' groups hold, folded or not.
pub(super) fn total(lines: &[Line]) -> usize {
    let counts = lines.iter().map(|one| match one {
        Line::Heading { count, .. } => *count,
        Line::Entry(_) => 0,
    });
    counts.sum()
}

pub(super) fn lines(app: &AppState, tab: &Opened, object: &Described) -> Vec<Line> {
    let mut uses = Vec::new();
    let mut used = Vec::new();
    if let Some(pod) = &object.pod {
        for one in &pod.uses {
            let open = uses_link(app, &tab.link, &one.tag, &one.name);
            let how = (one.how.join(" · "), Role::Muted);
            uses.push(entry(&one.tag, &one.name, Some(how), open));
        }
        for name in selecting(app, &tab.link, object) {
            let link = link_to(app, &tab.link, "Service", &name, true);
            used.push(entry("svc", &name, None, link));
        }
    }
    let groups = [
        ("owned by", owned(app, tab, object)),
        ("uses", uses),
        ("used by", used),
    ];
    let mut out = Vec::new();
    for (title, entries) in groups.into_iter().filter(|(_, all)| !all.is_empty()) {
        let shut = tab.folded(&key(title), false);
        let count = entries.len();
        out.push(Line::Heading { title, count, shut });
        if !shut {
            out.extend(entries.into_iter().map(|one| Line::Entry(Box::new(one))));
        }
    }
    out
}

fn key(title: &str) -> String {
    format!("relations/{title}")
}

fn entry(tag: &str, name: &str, status: Option<(String, Role)>, open: Option<Link>) -> Entry {
    Entry {
        depth: 0,
        tag: tag.to_string(),
        name: name.to_string(),
        status,
        open,
    }
}

/// Each owner from the top down.
fn owned(app: &AppState, tab: &Opened, object: &Described) -> Vec<Entry> {
    let owners = lineage(app, &tab.link, object);
    let mut out: Vec<Entry> = Vec::new();
    for (depth, (link, read)) in owners.iter().rev().enumerate() {
        let aside = read
            .as_ref()
            .and_then(|one| one.replicas)
            .map(|(ready, desired)| {
                let role = super::super::health::colour(groove_types::count_health(ready, desired));
                (format!("{ready}/{desired} ready"), role)
            });
        let tag = short(&link.kind.kind);
        out.push(Entry {
            depth,
            tag,
            name: link.name.clone(),
            status: aside,
            open: Some(link.clone()),
        });
    }
    out
}

/// One line, under the table's header when `header`.
pub(super) fn draw(ctx: &mut Ctx, rect: Rect, (line, header): (&Line, bool), last: Option<&str>) {
    let caret = Cell::fold(true, Role::Ghost);
    let columns = [
        grouped::plain("", Width::FitCell(&caret)),
        grouped::plain("KIND", Width::Fit("secret")),
        grouped::plain("NAME", Width::Fit("paxone-worker-5d6b9c7f4")),
        grouped::plain("STATUS", Width::Fill),
    ];
    let one = match line {
        Line::Heading { title, count, shut } => {
            let cells = vec![
                Cell::small(*title, Role::Text),
                Cell::small(count.to_string(), Role::Ghost),
            ];
            let heading = grouped::heading((&key(title), *shut), cells);
            grouped::single(ctx, &columns, header).one(ctx, rect, heading);
            return;
        }
        Line::Entry(one) => one,
    };
    let (said, role) = one.status.clone().unwrap_or((String::new(), Role::Muted));
    let cells = vec![
        Cell::small("", Role::Ghost),
        Cell::small(one.tag.as_str(), Role::Ghost),
        Cell::small("", Role::Text),
        Cell::small(said, role),
    ];
    let laid = grouped::single(ctx, &columns, header).one(ctx, rect, Shown::row(cells));
    let name = laid
        .cell(0, 2)
        .pad(Edges::across(ctx.tokens.md * one.depth as f32, 0.0));
    let role = if one.open.is_some() {
        Role::Accent
    } else {
        Role::Text
    };
    copy::Copied::new(&one.name, role, last)
        .open(one.open.clone())
        .draw(ctx, name);
}

/// `Deployment` reads `deploy`, as `kubectl` shortens it; anything else in lower case.
fn short(kind: &str) -> String {
    match kind {
        "Deployment" => "deploy".into(),
        "ReplicaSet" => "rs".into(),
        "StatefulSet" => "sts".into(),
        "DaemonSet" => "ds".into(),
        "CronJob" => "cj".into(),
        other => other.to_lowercase(),
    }
}

/// The object a pod's `uses` tag names.
fn uses_link(app: &AppState, from: &Link, tag: &str, name: &str) -> Option<Link> {
    let (kind, namespaced) = match tag {
        "sa" => ("ServiceAccount", true),
        "cm" => ("ConfigMap", true),
        "secret" => ("Secret", true),
        "pvc" => ("PersistentVolumeClaim", true),
        "node" => ("Node", false),
        _ => return None,
    };
    link_to(app, from, kind, name, namespaced)
}

fn link_to(app: &AppState, from: &Link, kind: &str, name: &str, namespaced: bool) -> Option<Link> {
    Some(Link {
        context: from.context.clone(),
        kind: served(app, &from.context, "", kind)?,
        namespace: from.namespace.clone().filter(|_| namespaced),
        name: name.to_string(),
    })
}
