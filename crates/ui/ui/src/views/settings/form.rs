//! The form: the section's rows, or every row the search finds with its section beside it.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use groove_controllers::agent_service::Terminal;
use groove_controllers::config_service::Preference;

mod agent;
mod clusters;

use self::agent::{routine, skill};
use super::SettingsUi;
use super::rows::{Row, Section, Value, rows};
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::offsets::listed;
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in, row_in};
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, picker, screen};

pub(super) fn draw(ctx: &mut Ctx, area: Rect, app: &AppState, settings: &SettingsUi) {
    let band = Spec::default().height(ctx.tokens.header);
    let [bar, area] = column_in(area, [band, Spec::fill()]);
    super::back(ctx, bar);
    let [head, mut body] = column_in(area.pad(Edges::all(ctx.tokens.md)), [band, Spec::fill()]);
    let query = settings.search.text();
    let searching = !query.is_empty();
    let shown: Vec<Row> = rows(app, settings)
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
    Label::new(&heading, ctx.styles.title(Role::Text)).draw(ctx, head);
    if !searching && settings.section == Section::Clusters {
        return clusters::draw(ctx, body, app);
    }
    let login = app.agent.login.as_ref();
    if let Some(terminal) = login.filter(|_| !searching && settings.section == Section::Setup) {
        let pane = super::login_pane(ctx.window, &ctx.tokens);
        body = body.until_y(pane.y);
        signing_in(ctx, pane, terminal);
    }
    let lines = lines(&shown, searching);
    let (row, heading) = (
        ctx.tokens.row + ctx.tokens.sm,
        ctx.tokens.row + ctx.tokens.md,
    );
    let height = |line: &Line| match line {
        Line::Group(_) => heading,
        Line::Setting(..) => row,
    };
    let scroller = (Scroller::Settings, settings.scroll);
    listed(
        ctx,
        body,
        scroller,
        &lines,
        height,
        |ctx, rect, line| match line {
            Line::Group(name) => group(ctx, rect, name),
            Line::Setting(one, ruled) => setting(ctx, rect, one, (searching, *ruled)),
        },
    );
}

/// One line of the form: a group's heading, or a row.
enum Line<'a> {
    Group(&'static str),
    /// A row, and whether a rule closes it: none while its block goes on under it.
    Setting(&'a Row, bool),
}

/// The rows, a heading wherever the group changes; none while a search mixes them.
fn lines(shown: &[Row], searching: bool) -> Vec<Line<'_>> {
    let mut out = Vec::new();
    let mut under = "";
    for (at, one) in shown.iter().enumerate() {
        if !searching && one.group != under {
            under = one.group;
            out.push(Line::Group(under));
        }
        let ruled = !shown.get(at + 1).is_some_and(Row::sub);
        out.push(Line::Setting(one, ruled));
    }
    out
}

/// A group's heading, at the foot of its gap.
fn group(ctx: &mut Ctx, rect: Rect, name: &str) {
    let [_, line] = column_in(rect, [Spec::fill(), Spec::default().height(ctx.tokens.row)]);
    Label::new(name, ctx.styles.label(Role::Faint)).draw(ctx, line);
}

/// The sign-in's terminal, which has the keys while it runs.
fn signing_in(ctx: &mut Ctx, pane: Rect, terminal: &Terminal) {
    groove_ui_kit::shape::ground(ctx, pane, Ground::Deep);
    ctx.hit(pane, Target::Login);
    let origin = (pane.x + ctx.tokens.sm, pane.y + ctx.tokens.sm);
    screen(ctx, pane, origin, (&terminal.screen(), true));
}

/// One row: its label, its group when a search mixes them, its value at the right.
fn setting(ctx: &mut Ctx, line: Rect, one: &Row, (searching, ruled): (bool, bool)) {
    if ruled {
        hairline(ctx, line, ctx.styles.line());
    }
    let [label, room] = row_in(
        line,
        [Spec::default().width(ctx.tokens.aside_mid), Spec::fill()],
    );
    let mut label_room = label;
    Label::new(&one.label, ctx.styles.body(Role::Muted)).left(ctx, &mut label_room, ctx.tokens.sm);
    if searching {
        let from = match one.group.is_empty() {
            true => one.section.label(),
            false => one.group,
        };
        Label::new(from, ctx.styles.small(Role::Ghost)).draw(ctx, label_room);
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
            let toggle = groove_ui_kit::widgets::Toggle::new(*on, Target::SetPreference(*flip));
            toggle.left(ctx, &mut room, sm);
        }
        Value::Count { shown, less, more } => stepped(ctx, room, shown, (*less, *more)),
        Value::State { shown, role, act } => {
            Label::new(shown, ctx.styles.body(*role)).left(ctx, &mut room, sm);
            if let Some((word, target)) = act {
                Button::new(word, target.clone(), Role::Muted, ground).left(ctx, &mut room, sm);
            }
        }
        Value::Input {
            shown,
            focused,
            target,
        } => input(ctx, room, (shown, *focused), target),
        Value::Picker {
            shown,
            role,
            target,
            act,
        } => picked(ctx, room, (shown, *role, target), act.as_ref()),
        Value::Choice(options) => {
            let each = options
                .iter()
                .map(|(word, held, pick)| (*word, *held, Target::SetPreference(*pick)));
            held_words(ctx, room, each, Role::Muted);
        }
        Value::Skill {
            id,
            on,
            said,
            deletes,
            asking,
        } => skill(ctx, room, (id, *on, said), (*deletes, *asking)),
        Value::Routine { .. } | Value::Switch { .. } => routine(ctx, room, value),
    }
}

/// A value as the picker that changes it, and the action beside it.
fn picked(
    ctx: &mut Ctx,
    mut room: Rect,
    (shown, role, target): (&str, Role, &Target),
    act: Option<&(&'static str, Target)>,
) {
    let (band, hover, ground, sm) = (
        ctx.styles.band(),
        ctx.styles.hover(),
        ctx.styles.ground(),
        ctx.tokens.sm,
    );
    picker(shown, target.clone(), role, band, hover).left(ctx, &mut room, sm);
    if let Some((word, at)) = act {
        Button::new(word, at.clone(), Role::Muted, ground).left(ctx, &mut room, sm);
    }
}

/// A count between the step down, when it has one, and the step up.
fn stepped(
    ctx: &mut Ctx,
    mut room: Rect,
    shown: &str,
    (less, more): (Option<Preference>, Preference),
) {
    let (ground, sm) = (ctx.styles.ground(), ctx.tokens.sm);
    if let Some(less) = less {
        Button::new("−", Target::SetPreference(less), Role::Muted, ground).left(ctx, &mut room, sm);
    }
    Label::new(shown, ctx.styles.code(Role::Text)).left(ctx, &mut room, sm);
    Button::new("+", Target::SetPreference(more), Role::Muted, ground).left(ctx, &mut room, sm);
}

/// A row of words, the ones held in the text's colour, the rest in `quiet`.
fn held_words(
    ctx: &mut Ctx,
    mut room: Rect,
    words: impl Iterator<Item = (&'static str, bool, Target)>,
    quiet: Role,
) {
    let (ground, sm) = (ctx.styles.ground(), ctx.tokens.sm);
    for (word, held, target) in words {
        let role = if held { Role::Text } else { quiet };
        Button::new(word, target, role, ground).left(ctx, &mut room, sm);
    }
}

/// A field on its own ground, raised while it has the keys.
fn input(ctx: &mut Ctx, room: Rect, (shown, focused): (&str, bool), target: &Target) {
    let sm = ctx.tokens.sm;
    let field = room.pad(Edges::across(0.0, sm));
    let (ground, role) = match focused {
        true => (Ground::Raised, Role::Text),
        false => (Ground::Band, Role::Muted),
    };
    groove_ui_kit::shape::ground(ctx, field, ground);
    ctx.hit(field, target.clone());
    let text = field.pad(Edges::across(sm, sm));
    Label::new(shown, ctx.styles.code(role)).draw(ctx, text);
}
