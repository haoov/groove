//! The session: the bar of its title and tabs, the mode in the
//! middle, its sidebar at the right, and the terminals' drawer at the middle's foot.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Edges;

use super::{Mode, agent_pane, header, manual};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::layout::{Spec, row_in};
use groove_ui_kit::widgets::{Side, Tab, tab_group};

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, open: &Open) {
    match ui.session.mode {
        Mode::Agent => agent_pane::draw(ctx, app, ui),
        Mode::Overview => super::overview::moded::draw(ctx, app, ui, ctx.app.layout.agent),
        Mode::Diff if ui.session.reviewing => {
            let gutter = Edges::xy(ctx.tokens.xl, ctx.tokens.sm);
            super::review::draw(ctx, app, ui, ctx.app.layout.agent.pad(gutter))
        }
        Mode::Diff => super::diff::moded::draw(ctx, app, ui, ctx.app.layout.agent),
        Mode::Files => super::diff::moded::file(ctx, app, ui, ctx.app.layout.agent),
        Mode::Kubernetes => super::resources::moded::draw(ctx, app, ui, ctx.app.layout.agent),
    }
    super::working::draw(ctx, app, open);
    ctx.layer();
    bar(ctx, app, ui, open);
    let (aside, shown) = (ctx.app.layout.sidebar, !ui.session.folded);
    super::aside::draw(ctx, app, ui, open, (aside, shown));
    manual::draw(ctx, app, ui, &open.session.id);
    if crate::views::rail::over(app, ui) {
        ctx.layer();
        crate::views::rail::windows(ctx, app, ui);
    }
}

/// The session's title at the left, its tabs at the right, the mode up held.
fn bar(ctx: &mut Ctx, app: &AppState, ui: &Ui, open: &Open) {
    let line = ctx.app.layout.top;
    let mut room = line;
    let shown = Mode::ALL
        .into_iter()
        .filter(|one| *one != Mode::Kubernetes || open.holds_clusters());
    let tabs: Vec<Tab<'static, Target>> = shown
        .map(|one| {
            Tab::new(one.label(), Target::ModeTab(one), one == ui.session.mode)
                .text(groove_ui_kit::widgets::Text::Body)
        })
        .collect();
    tab_group(ctx, &mut room, tabs, (Side::Right, ctx.tokens.md));
    let page = app
        .task
        .worked(&open.session)
        .and_then(|one| one.url.as_deref());
    let kept = header::worktree_wide(ctx, open).min(room.w / 2.0);
    let fixed = |width: f32| Spec::default().width(width);
    let [title, _] = row_in(room, [Spec::fill(), fixed(kept)]);
    let rest = header::titled_rest(ctx, title, open, page);
    let [_, tail] = row_in(room, [fixed(rest.x - room.x), Spec::fill()]);
    header::worktree_of(ctx, tail, open);
}
