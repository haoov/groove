//! What the object stands among, in three columns: what owns it, what it uses, what uses it.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::Described;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_of, row_in};
use groove_ui_kit::text::Label;

use super::super::opened::{Link, Opened, lineage, selecting, served};
use super::copy;
use crate::ctx::Ctx;

/// One line of a column: a tag, a name, what follows it, the tab it opens; set in by `depth`.
pub(super) struct Entry {
    depth: usize,
    tag: String,
    name: String,
    aside: Option<(String, Role)>,
    open: Option<Link>,
}

pub(super) struct Columns {
    owned: Vec<Entry>,
    uses: Vec<Entry>,
    used: Vec<Entry>,
}

impl Columns {
    /// How many lines the tallest column holds, its title included.
    pub(super) fn rows(&self) -> usize {
        1 + self.owned.len().max(self.uses.len()).max(self.used.len())
    }
}

pub(super) fn of(app: &AppState, tab: &Opened, object: &Described) -> Columns {
    let mut uses = Vec::new();
    let mut used = Vec::new();
    if let Some(pod) = &object.pod {
        for (tag, name) in pod.uses.iter().filter(|(tag, _)| tag != "node") {
            uses.push(plain(tag, name, uses_link(app, &tab.link, tag, name)));
        }
        for name in selecting(app, &tab.link, object) {
            let link = link_to(app, &tab.link, "Service", &name, true);
            used.push(plain("svc", &name, link));
        }
    }
    Columns {
        owned: owned(app, tab, object),
        uses,
        used,
    }
}

fn plain(tag: &str, name: &str, open: Option<Link>) -> Entry {
    Entry {
        depth: 0,
        tag: tag.to_string(),
        name: name.to_string(),
        aside: None,
        open,
    }
}

/// Each owner from the top down, then the object itself.
fn owned(app: &AppState, tab: &Opened, object: &Described) -> Vec<Entry> {
    let owners = lineage(app, &tab.link, object);
    let mut out: Vec<Entry> = Vec::new();
    for (depth, (link, read)) in owners.iter().rev().enumerate() {
        let aside = read
            .as_ref()
            .and_then(|one| one.replicas)
            .map(|(ready, desired)| {
                let role = if ready >= desired {
                    Role::Ok
                } else {
                    Role::Warn
                };
                (format!("{ready}/{desired}"), role)
            });
        let tag = short(&link.kind.kind);
        out.push(Entry {
            depth,
            tag,
            name: link.name.clone(),
            aside,
            open: Some(link.clone()),
        });
    }
    let tag = short(&tab.link.kind.kind);
    out.push(Entry {
        depth: owners.len(),
        tag,
        name: "this".into(),
        aside: None,
        open: None,
    });
    out
}

pub(super) fn draw(ctx: &mut Ctx, rect: Rect, columns: &Columns, last: Option<&str>) {
    let [owned, uses, used] = row_in(rect, [Spec::fill(), Spec::fill(), Spec::fill()]);
    column(ctx, owned, ("owned by", last), &columns.owned);
    column(ctx, uses, ("uses", last), &columns.uses);
    column(ctx, used, ("used by", last), &columns.used);
}

fn column(ctx: &mut Ctx, rect: Rect, (title, last): (&str, Option<&str>), entries: &[Entry]) {
    if entries.is_empty() {
        return;
    }
    let row = Spec::default().height(ctx.tokens.row);
    let lines = column_of(rect, &vec![row; entries.len() + 1]);
    Label::new(title, ctx.styles.small(Role::Ghost)).draw(ctx, lines[0]);
    let style = ctx.styles.small(Role::Ghost);
    let widest = entries
        .iter()
        .map(|one| ctx.measure(&one.tag, &style))
        .fold(0.0, f32::max);
    for (line, one) in lines[1..].iter().zip(entries) {
        entry(ctx, *line, one, (widest + ctx.tokens.sm, last));
    }
}

fn entry(ctx: &mut Ctx, rect: Rect, one: &Entry, (tags, last): (f32, Option<&str>)) {
    let indent = ctx.tokens.md * one.depth as f32;
    let [_, tag, rest] = row_in(
        rect,
        [
            Spec::default().width(indent),
            Spec::default().width(tags),
            Spec::fill(),
        ],
    );
    Label::new(&one.tag, ctx.styles.small(Role::Ghost)).draw(ctx, tag);
    let mut room = rest;
    let role = if one.open.is_some() {
        Role::Accent
    } else {
        Role::Text
    };
    let style = ctx.styles.small(role);
    match one.open.is_some() {
        true => {
            copy::Copied::new(&one.name, role, last)
                .open(one.open.clone())
                .left(ctx, &mut room, ctx.tokens.sm);
        }
        false => {
            Label::new(&one.name, style).left(ctx, &mut room, ctx.tokens.sm);
        }
    }
    if let Some((text, role)) = &one.aside {
        Label::new(text, ctx.styles.small(*role)).draw(ctx, room);
    }
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
