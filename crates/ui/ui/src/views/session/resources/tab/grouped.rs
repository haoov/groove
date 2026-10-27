//! The list's table under headings that fold: an Application's resources by kind, its sync by stage,
//! and a row at a time in the described view.

use std::collections::BTreeMap;

use groove_gfx::Rect;
use groove_ui_kit::widgets::{Cell, Column, Rows, Shown, Sorted, Table, Width};

use super::super::opened::Opened;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};

/// A column's label, and the text it is as wide as until dragged; the last fills.
pub(super) type Columned = (&'static str, &'static str);

pub(super) fn draw<'a>(
    ctx: &mut Ctx,
    body: Rect,
    (tab, dragged): (&Opened, &BTreeMap<String, f32>),
    (columns, count): (&[Columned], usize),
    shown: impl FnMut(usize) -> Shown<'a, Target>,
) {
    let last = columns.len().saturating_sub(1);
    let columns: Vec<Column<'_, Target>> = columns
        .iter()
        .enumerate()
        .map(|(at, (label, sample))| Column {
            label,
            width: match (dragged.get(*label), at == last) {
                (Some(wide), _) => Width::Fixed(*wide),
                (None, true) => Width::Fill,
                (None, false) => Width::FitBody(sample),
            },
            end: false,
            sort: Some(Target::ResourceSort(label.to_string())),
            edge: Some(Target::ResourceEdge(label.to_string())),
        })
        .collect();
    let sorted = tab.sort.as_ref().and_then(|(label, descending)| {
        let column = columns.iter().position(|one| one.label == label)?;
        Some(Sorted {
            column,
            descending: *descending,
        })
    });
    let table = Table {
        columns: &columns,
        rows: Rows {
            first: ctx.tokens.row,
            under: 0.0,
            ruled: false,
            carded: false,
        },
        count,
        offset: tab.scroll,
        selected: None,
        sorted: Some(sorted.unwrap_or(Sorted::NONE)),
    };
    let laid = table.draw(ctx, body, shown);
    ctx.app.hits.scrolls(Scroller::Resources, laid.extent);
}

/// A group's heading behind its fold caret, which folds it on a click.
pub(super) fn heading<'a>((key, shut): (&str, bool), cells: Vec<Cell<'a>>) -> Shown<'a, Target> {
    Shown::band(cells)
        .fold(!shut)
        .target(Target::ResourceFold(key.to_string(), shut))
}

/// The order a column picks, rows of one group compared by `key`.
pub(super) fn order<R>(rows: &mut [R], tab: &Opened, key: impl Fn(&R, &str) -> String) {
    let Some((label, descending)) = &tab.sort else {
        return;
    };
    rows.sort_by(|a, b| {
        let order = key(a, label).cmp(&key(b, label));
        if *descending { order.reverse() } else { order }
    });
}

/// The table a row stands alone in, its header above it when `header`.
pub(super) fn single<'a>(
    ctx: &Ctx,
    columns: &'a [Column<'a, Target>],
    header: bool,
) -> Table<'a, Target> {
    Table {
        columns,
        rows: Rows {
            first: ctx.tokens.row,
            under: 0.0,
            ruled: false,
            carded: false,
        },
        count: 1,
        offset: 0.0,
        selected: None,
        sorted: header.then_some(Sorted::NONE),
    }
}
