//! Settings › Clusters: the contexts Groove knows as a table, then the ones the kubeconfig adds.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::{ClusterChange, ClusterConfig, KubeContext, Login};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Act, Cell, Column, Rows, Shown, Sorted, Table, Width};

use crate::ctx::Ctx;
use crate::hit::Target;

pub(super) fn draw(ctx: &mut Ctx, body: Rect, app: &AppState) {
    let added = app.config.clusters();
    let found = app.cluster.found.as_deref().unwrap_or_default();
    let rest: Vec<&KubeContext> = found
        .iter()
        .filter(|one| app.config.cluster(&one.name).is_none())
        .collect();
    let (line, heading) = (
        ctx.tokens.row + ctx.tokens.sm,
        ctx.tokens.row + ctx.tokens.md,
    );
    let tall = |rows: usize| Spec::default().height(line * (rows.max(1) + 1) as f32);
    let [first, contexts, second, others, _] = column_in(
        body,
        [
            Spec::default().height(heading),
            tall(added.len()),
            Spec::default().height(heading),
            tall(rest.len()),
            Spec::fill(),
        ],
    );
    group(ctx, first, "Contexts");
    match added.is_empty() {
        true => quiet(
            ctx,
            contexts,
            "none added yet: add one from your kubeconfig below",
        ),
        false => known(ctx, contexts, app, added),
    }
    group(ctx, second, "In your kubeconfig, not added");
    match (app.cluster.found.is_none(), rest.is_empty()) {
        (true, _) => quiet(ctx, others, "reading the kubeconfig…"),
        (false, true) => quiet(ctx, others, "every context is added"),
        (false, false) => unknown(ctx, others, &rest),
    }
}

fn group(ctx: &mut Ctx, rect: Rect, name: &str) {
    let [_, line] = column_in(rect, [Spec::fill(), Spec::default().height(ctx.tokens.row)]);
    Label::new(name, ctx.styles.label(Role::Faint)).draw(ctx, line);
}

fn quiet(ctx: &mut Ctx, rect: Rect, said: &str) {
    let [line, _] = column_in(rect, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    Label::new(said, ctx.styles.body(Role::Ghost)).draw(ctx, line);
}

fn column<'a>(label: &'a str, width: Width<'a>) -> Column<'a, Target> {
    Column {
        label,
        width,
        end: false,
        sort: None,
    }
}

fn table<'a>(columns: &'a [Column<'a, Target>], count: usize, line: f32) -> Table<'a, Target> {
    Table {
        columns,
        rows: Rows {
            first: line,
            under: 0.0,
            ruled: true,
        },
        count,
        offset: 0.0,
        selected: None,
        sorted: Some(Sorted::NONE),
    }
}

/// One line an added context: its hue and context, its switches, its login and its removal.
fn known(ctx: &mut Ctx, rect: Rect, app: &AppState, added: &[ClusterConfig]) {
    let contexts = widest(added.iter().map(|one| one.context.clone()));
    let columns = [
        column("", Width::Fit("")),
        column("context", Width::Fit(&contexts)),
        column("read-only", Width::Fit("read-only")),
        column("argo hub", Width::Fit("argo hub")),
        column(
            "login",
            Width::Fit("sign-in refused · log in from a terminal"),
        ),
        column("", Width::Fit("remove")),
        column("", Width::Fill),
    ];
    let line = ctx.tokens.row + ctx.tokens.sm;
    let table = table(&columns, added.len(), line);
    table.draw(ctx, rect, |at| context(app, &added[at]));
}

/// The longest of `texts`, which a column fits.
fn widest(texts: impl Iterator<Item = String>) -> String {
    texts
        .max_by_key(|one| one.chars().count())
        .unwrap_or_default()
}

fn context<'a>(app: &AppState, one: &'a ClusterConfig) -> Shown<'a, Target> {
    let name = one.context.clone();
    let switch = |on, change| Some(Act::Toggle(on, Target::ClusterSet(name.clone(), change)));
    let (said, role) = login(app, &one.context);
    let hue = Role::Hue(one.hue);
    let blank = || Cell::small("", Role::Text);
    Shown {
        cells: vec![
            blank().mark(Mark::Context, 0, hue),
            Cell::label(one.context.as_str(), hue),
            blank(),
            blank(),
            Cell::small(said, role),
            blank(),
        ],
        under: Vec::new(),
        target: Some(Target::ClusterRow(name.clone())),
        acts: vec![
            None,
            None,
            switch(one.read_only, ClusterChange::ReadOnly(!one.read_only)),
            switch(one.argo_hub, ClusterChange::ArgoHub(!one.argo_hub)),
            None,
            Some(Act::Button("remove", Target::ClusterRemove(name.clone()))),
        ],
    }
}

/// What the last check found; a refusal is fixed from a terminal, never from here.
fn login(app: &AppState, context: &str) -> (String, Role) {
    if app.cluster.checking(context) {
        return ("checking…".into(), Role::Working);
    }
    match app.cluster.login(context) {
        None => ("not checked".into(), Role::Muted),
        Some(Login::SignedIn { version }) => (format!("signed in · {version}"), Role::Ok),
        Some(Login::Refused(_)) => (
            "sign-in refused · log in from a terminal".into(),
            Role::Attention,
        ),
        Some(Login::Unreachable(_)) => ("unreachable".into(), Role::Bad),
        Some(Login::Failed(why)) => (why.clone(), Role::Bad),
    }
}

/// One line a context the kubeconfig names that Groove does not know yet.
fn unknown(ctx: &mut Ctx, rect: Rect, rest: &[&KubeContext]) {
    let contexts = widest(rest.iter().map(|one| one.name.clone()));
    let servers = widest(rest.iter().filter_map(|one| one.server.clone()));
    let columns = [
        column("context", Width::Fit(&contexts)),
        column("server", Width::Fit(&servers)),
        column("", Width::Fit("add")),
        column("", Width::Fill),
    ];
    let line = ctx.tokens.row + ctx.tokens.sm;
    let table = table(&columns, rest.len(), line);
    table.draw(ctx, rect, |at| {
        let one = rest[at];
        let add = Target::ClusterAdd(one.name.clone());
        Shown {
            cells: vec![
                Cell::small(one.name.as_str(), Role::Text),
                Cell::small(one.server.as_deref().unwrap_or_default(), Role::Muted),
                Cell::small("", Role::Text),
            ],
            under: Vec::new(),
            target: Some(Target::ClusterRow(one.name.clone())),
            acts: vec![None, None, Some(Act::Button("add", add))],
        }
    });
}
