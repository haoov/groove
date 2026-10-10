//! The parts the described view is made of: a section's heading, the conditions, the facts as a
//! grid, a key and its value, an event.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::{Described, EventRow, PodPart, Timestamp};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_of, row_in};
use groove_ui_kit::text::{Label, ago};

use super::super::list::plan::colour;
use super::super::opened::{Link, Opened, Section, helm_release, lineage, served};
use super::copy;
use crate::ctx::Ctx;
use crate::hit::Target;

/// How many facts stand on one line of the summary.
pub(super) const ACROSS: usize = 3;

/// One fact of the summary: its key, its value in its colour, the tab it opens.
pub(super) struct Fact {
    key: &'static str,
    value: String,
    role: Role,
    open: Option<Link>,
}

impl Fact {
    pub(super) fn new(key: &'static str, value: String, role: Role) -> Self {
        Self {
            key,
            value,
            role,
            open: None,
        }
    }

    /// What a click copies: a name or an address, never a count or an age.
    fn copied(&self) -> Option<&str> {
        let head = self.value.split(" · ").next();
        match self.key {
            "node" | "pod ip" | "qos" | "repo" | "path" | "target" | "values" | "synced" => {
                Some(&self.value)
            }
            "helm" | "priority" | "destination" => head,
            _ => None,
        }
    }
}

/// The fold mark, the section's name and what it holds; a click folds it.
pub(super) fn heading(ctx: &mut Ctx, rect: Rect, (section, shut): (Section, bool), count: &str) {
    let mut room = rect;
    let mark = if shut { "▸" } else { "▾" };
    Label::new(mark, ctx.styles.small(Role::Ghost)).left(ctx, &mut room, ctx.tokens.sm);
    let title = format!("{section:?}").to_uppercase();
    Label::new(&title, ctx.styles.small(Role::Faint)).left(ctx, &mut room, ctx.tokens.sm);
    Label::new(count, ctx.styles.small(Role::Ghost)).draw(ctx, room);
    ctx.hit(rect, Target::ResourceSection(section));
}

/// Every condition on one line: a mark, its name, and why and since when where it is not met.
pub(super) fn conditions(ctx: &mut Ctx, rect: Rect, pod: &PodPart) {
    let mut room = rect;
    for one in &pod.conditions {
        let (mark, role, name) = match one.met {
            true => ("✓", Role::Ok, Role::Muted),
            false => ("✗", Role::Bad, Role::Text),
        };
        Label::new(mark, ctx.styles.small(role)).left(ctx, &mut room, ctx.tokens.xs);
        Label::new(&one.kind, ctx.styles.small(name)).left(ctx, &mut room, ctx.tokens.xs);
        let since = one.since.map(|at| ago(at.age_at(ctx.now)));
        let why = [one.reason.clone().filter(|_| !one.met), since];
        let why: Vec<String> = why.into_iter().flatten().collect();
        let why = why.join(" · ");
        Label::new(&why, ctx.styles.small(Role::Ghost)).left(ctx, &mut room, ctx.tokens.lg);
    }
}

/// The summary's facts: a pod's state and place, a workload's replicas; the Helm release.
pub(super) fn facts(app: &AppState, tab: &Opened, object: &Described, now: Timestamp) -> Vec<Fact> {
    let mut out = Vec::new();
    let mut add = |key, value: String, role, open| {
        out.push(Fact {
            key,
            value,
            role,
            open,
        })
    };
    if let Some(pod) = &object.pod {
        pod_facts(app, tab, pod, now, &mut add);
    } else if let Some((ready, desired)) = object.replicas {
        let role = if ready >= desired {
            Role::Ok
        } else {
            Role::Warn
        };
        add("ready", format!("{ready}/{desired}"), role, None);
    }
    if let Some(created) = object.created {
        add(
            "created",
            format!("{} ago", ago(created.age_at(now))),
            Role::Text,
            None,
        );
    }
    if let Some((namespace, name)) = helm_release(object, &lineage(app, &tab.link, object)) {
        let release = (tab.link.context.clone(), namespace, name.clone());
        let revision = app.cluster.store.follows.helm(&release).flatten();
        let value = revision.map_or(name.clone(), |one| format!("{name} · rev {one}"));
        add("helm", value, Role::Text, None);
    }
    out
}

fn pod_facts(
    app: &AppState,
    tab: &Opened,
    pod: &PodPart,
    now: Timestamp,
    add: &mut dyn FnMut(&'static str, String, Role, Option<Link>),
) {
    let health = groove_types::status_health(&pod.status).unwrap_or(groove_types::Health::Waiting);
    add("status", pod.status.clone(), colour(health), None);
    let running = pod.containers.iter().filter(|one| !one.init);
    let (ready, all) = running.fold((0, 0), |(ready, all), one| {
        (ready + usize::from(one.ready), all + 1)
    });
    add(
        "ready",
        format!("{ready}/{all}"),
        if ready == all { Role::Ok } else { Role::Bad },
        None,
    );
    let restarts: u32 = pod.containers.iter().map(|one| one.restarts).sum();
    let last = pod
        .containers
        .iter()
        .filter_map(|one| one.last.as_ref()?.at)
        .max();
    let when = last
        .map(|at| format!(" · last {} ago", ago(at.age_at(now))))
        .unwrap_or_default();
    add(
        "restarts",
        format!("{restarts}{when}"),
        if restarts > 0 { Role::Bad } else { Role::Text },
        None,
    );
    placed(app, tab, pod, now, add);
}

/// Where the pod runs and since when.
fn placed(
    app: &AppState,
    tab: &Opened,
    pod: &PodPart,
    now: Timestamp,
    add: &mut dyn FnMut(&'static str, String, Role, Option<Link>),
) {
    if let Some(node) = &pod.node {
        let open = served(app, &tab.link.context, "", "Node").map(|kind| Link {
            context: tab.link.context.clone(),
            kind,
            namespace: None,
            name: node.clone(),
        });
        add("node", node.clone(), Role::Accent, open);
    }
    let facts = [("pod ip", pod.ip.clone()), ("qos", pod.qos.clone())];
    for (key, value) in facts
        .into_iter()
        .filter_map(|(key, value)| Some((key, value?)))
    {
        add(key, value, Role::Text, None);
    }
    if let Some(started) = pod.started {
        add(
            "started",
            format!("{} ago", ago(started.age_at(now))),
            Role::Text,
            None,
        );
    }
    if let Some((class, value)) = &pod.priority {
        add("priority", format!("{class} · {value}"), Role::Muted, None);
    }
}

/// The facts in rows of `ACROSS`, each its key then its value.
pub(super) fn grid(ctx: &mut Ctx, rect: Rect, facts: &[Fact], last: Option<&str>) {
    let rows = facts.len().div_ceil(ACROSS);
    let lines = column_of(rect, &vec![Spec::default().height(ctx.tokens.row); rows]);
    for (line, chunk) in lines.iter().zip(facts.chunks(ACROSS)) {
        let cells = row_in(*line, [Spec::fill(), Spec::fill(), Spec::fill()]);
        for (cell, fact) in cells.iter().zip(chunk) {
            let [key, value] = row_in(
                *cell,
                [Spec::default().width(ctx.tokens.aside_near), Spec::fill()],
            );
            Label::new(fact.key, ctx.styles.small(Role::Ghost)).draw(ctx, key);
            match fact.copied() {
                Some(part) => {
                    copy::Copied::new(&fact.value, fact.role, last)
                        .open(fact.open.clone())
                        .copies(part)
                        .draw(ctx, value);
                }
                None => {
                    Label::new(&fact.value, ctx.styles.small(fact.role)).draw(ctx, value);
                }
            }
        }
    }
}

/// A key in the key column, `keys` wide, then its value.
pub(super) fn pair(
    ctx: &mut Ctx,
    rect: Rect,
    (key, value): &(String, String),
    (keys, last): (f32, Option<&str>),
) {
    let [k, v] = row_in(rect, [Spec::default().width(keys), Spec::fill()]);
    let copied = |text, role| copy::Copied::new(text, role, last);
    copied(key, Role::Ghost).draw(ctx, k);
    copied(value, Role::Text).draw(ctx, v);
}

/// One event: a warning's mark, its reason, how often, how long ago, and what it says.
pub(super) fn event(ctx: &mut Ctx, rect: Rect, row: &EventRow, last: Option<&str>) {
    let narrow = Spec::default().width(ctx.tokens.lg * 2.0);
    let wide = Spec::default().width(ctx.tokens.aside_mid);
    let mark = Spec::default().width(ctx.tokens.icon);
    let [warned, reason, count, age, message] =
        row_in(rect, [mark, wide, narrow, narrow, Spec::fill()]);
    if row.warning {
        Label::new("⚠", ctx.styles.small(Role::Warn)).draw(ctx, warned);
    }
    let role = if row.warning { Role::Text } else { Role::Muted };
    Label::new(&row.reason, ctx.styles.small(role)).draw(ctx, reason);
    Label::new(&format!("×{}", row.count), ctx.styles.small(Role::Ghost)).draw(ctx, count);
    let when = row
        .last
        .map(|at| ago(at.age_at(ctx.now)))
        .unwrap_or_default();
    Label::new(&when, ctx.styles.small(Role::Ghost)).draw(ctx, age);
    copy::Copied::new(&row.message, role, last).draw(ctx, message);
}
