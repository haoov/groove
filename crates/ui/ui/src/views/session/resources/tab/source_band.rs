//! A source on the sync table's band: what it is, where it reads from, what it follows.

use groove_gfx::Rect;
use groove_types::AppSource;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, row_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Cell, Table};

use super::super::opened::Opens;
use super::{copy, grouped};
use crate::ctx::Ctx;
use crate::hit::Target;

/// A source: what it is, where it reads from, which opens its files when git, what it follows.
pub(super) struct Source {
    at: usize,
    kind: String,
    address: String,
    opens: Option<Opens>,
    details: String,
    shut: bool,
}

/// `SOURCE 1  git  gitlab.com/deploy · main · envs/acc` on the band; a click folds it.
pub(super) fn draw(
    ctx: &mut Ctx,
    (table, rect): (Table<'_, Target>, Rect),
    one: &Source,
    last: Option<&str>,
) {
    let cells = vec![
        Cell::small(format!("SOURCE {}", one.at + 1), Role::Text),
        Cell::small(one.kind.as_str(), Role::Muted),
    ];
    let x = after(ctx, &cells);
    let band = grouped::heading((&super::revisions::key(one.at), one.shut), cells);
    let line = table.one(ctx, rect, band).across(0);
    let [_, mut room] = row_in(line, [Spec::default().width(x), Spec::fill()]);
    let role = match one.opens.is_some() {
        true => Role::Accent,
        false => Role::Text,
    };
    let copied = copy::Copied::new(&one.address, role, last).opens(one.opens.clone());
    copied.left(ctx, &mut room, ctx.tokens.sm);
    Label::new(&one.details, ctx.styles.small(Role::Faint)).draw(ctx, room);
}

/// Where a band's text goes on past `cells`, from the band's left: its caret, then each cell.
fn after(ctx: &mut Ctx, cells: &[Cell]) -> f32 {
    let (md, sm) = (ctx.tokens.md, ctx.tokens.sm);
    let caret = ctx.tokens.small + ctx.tokens.xs;
    let drawn: f32 = cells.iter().map(|one| one.wide(ctx) + sm).sum();
    md + caret + drawn
}

pub(super) fn source(one: &AppSource, at: usize, shut: bool) -> Source {
    let kind = match &one.chart {
        Some(chart) => format!("chart {chart}"),
        None => "git".into(),
    };
    let opens = one.git().then(|| Opens::Repo {
        repo: one.repo.clone(),
        target: one.target.clone(),
    });
    let mut details = vec![String::new(), one.target.clone()];
    details.extend(one.path.clone());
    if !one.values.is_empty() {
        details.push(format!("values {}", one.values.join(", ")));
    }
    Source {
        at,
        kind,
        address: bare(&one.repo),
        opens,
        details: details.join(" · ").trim_start().to_string(),
        shut,
    }
}

/// A repository's address without its scheme, its credentials and `.git`.
fn bare(repo: &str) -> String {
    let path = repo.split_once("://").map_or(repo, |(_, rest)| rest);
    let host = path.find('/').unwrap_or(path.len());
    let path = match path[..host].rfind('@') {
        Some(at) => &path[at + 1..],
        None => path,
    };
    path.trim_end_matches(".git").to_string()
}
