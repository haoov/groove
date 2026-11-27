//! The kinds the contexts in scope serve, by heading, as the Kubernetes sidebar lists them.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{KindHeading, KubeKind};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::lit_cut;
use groove_ui_kit::widgets::{Heading, glass_hoverable};

use super::scope;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;

/// One line of the sidebar: a heading, or a kind under it; each a stop of the arrows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// A heading, and whether it stands open.
    Heading(KindHeading, bool),
    Kind(KubeKind),
}

/// A heading that folds, or a kind on the held ground while it is the one `picked`.
pub(super) fn kind_line(ctx: &mut Ctx, line: Rect, one: &Line, picked: Option<&KubeKind>) {
    match one {
        Line::Heading(group, open) => {
            glass_hoverable(ctx, line, Target::ResourceGroup(group.clone()));
            Heading::new(group.label()).fold(*open).draw(ctx, line)
        }
        Line::Kind(kind) => named(ctx, line, kind, picked == Some(kind)),
    }
}

/// One kind at a file's depth: listed, on the held ground between the rules.
fn named(ctx: &mut Ctx, line: Rect, kind: &KubeKind, picked: bool) {
    glass_hoverable(ctx, line, Target::ResourceKind(kind.clone()));
    if picked {
        groove_ui_kit::widgets::glass_chosen(ctx, line);
    }
    let role = if picked { Role::Text } else { Role::Muted };
    let style = ctx.styles.body(role);
    let at = crate::views::session::files::text_at(ctx, ctx.tokens.md * 2.0);
    let room = line.pad(Edges::across(0.0, ctx.tokens.md));
    let label = super::list::plural(kind);
    lit_cut(
        ctx,
        (room, at),
        &label,
        style,
        (&[], ctx.styles.body(Role::Accent)),
    );
}

/// Every watchable kind of `contexts`, once each.
pub fn served<'a>(app: &'a AppState, contexts: Vec<&str>) -> Vec<&'a KubeKind> {
    let mut kinds: Vec<&KubeKind> = Vec::new();
    for context in contexts {
        let served = app.cluster.store.kinds(context).unwrap_or_default();
        for kind in served.iter().filter(|one| one.watchable) {
            if !kinds
                .iter()
                .any(|one| one.group == kind.group && one.kind == kind.kind)
            {
                kinds.push(kind);
            }
        }
    }
    kinds
}

/// Every kind served, under its heading.
fn lines(app: &AppState, open: &Open, ui: &Ui) -> Vec<Line> {
    let held = &ui.session.resources;
    let kinds = served(app, scope::shown_contexts(open, held));
    let mut headed: std::collections::BTreeMap<KindHeading, Vec<&KubeKind>> = Default::default();
    for one in kinds {
        headed.entry(one.heading()).or_default().push(one);
    }
    let mut out = Vec::new();
    for (heading, mut under) in headed {
        under.sort_by(|a, b| a.kind.cmp(&b.kind));
        let open = !held.folded.contains(&heading);
        out.push(Line::Heading(heading, open));
        if open {
            out.extend(under.into_iter().map(|one| Line::Kind(one.clone())));
        }
    }
    out
}

/// The sidebar's lines as it shows them: what the arrows step through.
pub fn shown(app: &AppState, ui: &Ui) -> Vec<Line> {
    app.session
        .selected()
        .map(|open| lines(app, open, ui))
        .unwrap_or_default()
}
