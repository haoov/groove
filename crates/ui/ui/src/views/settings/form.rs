//! The form: the section's rows, or every row the search finds with its section beside it.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use super::SettingsUi;
use super::rows::{Row, Value, rows};
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::Word;

pub(super) fn draw(ctx: &mut Ctx, mut area: Rect, app: &AppState, settings: &SettingsUi) {
    super::back(ctx, area.take_top(ctx.tokens.header));
    let mut body = area.pad(Edges::all(ctx.tokens.md));
    let query = settings.search.text();
    let searching = !query.is_empty();
    let shown: Vec<Row> = rows(app)
        .into_iter()
        .filter(|row| match searching {
            true => row.matches(query),
            false => row.section == settings.section,
        })
        .collect();
    let heading = match searching {
        true => format!("{} found", shown.len()),
        false => settings.section.label().to_string(),
    };
    let head = body.take_top(ctx.tokens.header);
    Label::new(&heading, ctx.styles.title(Role::Text)).draw(ctx, head);
    for one in &shown {
        let line = body.take_top(ctx.tokens.row + ctx.tokens.sm);
        setting(ctx, line, one, searching);
    }
}

/// One row: its label, its section when a search mixes them, its value at the right.
fn setting(ctx: &mut Ctx, line: Rect, one: &Row, searching: bool) {
    hairline(ctx, line, ctx.styles.line());
    let mut room = line;
    let label = room.take_left(ctx.tokens.aside_mid);
    let mut label_room = label;
    Label::new(one.label, ctx.styles.body(Role::Muted)).left(ctx, &mut label_room, ctx.tokens.sm);
    if searching {
        Label::new(one.section.label(), ctx.styles.small(Role::Ghost)).draw(ctx, label_room);
    }
    value(ctx, room, &one.value);
}

fn value(ctx: &mut Ctx, mut room: Rect, value: &Value) {
    let (ground, sm) = (ctx.styles.ground(), ctx.tokens.sm);
    match value {
        Value::Text { text, mono } => {
            let style = match mono {
                true => ctx.styles.code(Role::Text),
                false => ctx.styles.body(Role::Text),
            };
            Label::new(text, style).draw(ctx, room);
        }
        Value::Toggle { on, flip } => {
            let (word, role) = match on {
                true => ("on", Role::Text),
                false => ("off", Role::Muted),
            };
            let target = Target::SetPreference(*flip);
            Word::new(word, target, role, ground).left(ctx, &mut room, sm);
        }
        Value::Count { shown, less, more } => {
            if let Some(less) = less {
                Word::new("−", Target::SetPreference(*less), Role::Muted, ground)
                    .left(ctx, &mut room, sm);
            }
            Label::new(shown, ctx.styles.code(Role::Text)).left(ctx, &mut room, sm);
            Word::new("+", Target::SetPreference(*more), Role::Muted, ground)
                .left(ctx, &mut room, sm);
        }
        Value::Choice(options) => {
            for (label, held, pick) in options {
                let role = match held {
                    true => Role::Text,
                    false => Role::Muted,
                };
                Word::new(label, Target::SetPreference(*pick), role, ground)
                    .left(ctx, &mut room, sm);
            }
        }
    }
}
