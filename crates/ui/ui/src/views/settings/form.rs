//! The form: the section's rows, or every row the search finds with its section beside it.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use groove_controllers::agent_service::Terminal;
use groove_controllers::config_service::Preference;

use super::SettingsUi;
use super::rows::{Row, Section, Value, rows};
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::offsets::listed;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, picker, screen};

pub(super) fn draw(ctx: &mut Ctx, mut area: Rect, app: &AppState, settings: &SettingsUi) {
    super::back(ctx, area.take_top(ctx.tokens.header));
    let mut body = area.pad(Edges::all(ctx.tokens.md));
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
    let head = body.take_top(ctx.tokens.header);
    Label::new(&heading, ctx.styles.title(Role::Text)).draw(ctx, head);
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
        Line::Setting(_) => row,
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
            Line::Setting(one) => setting(ctx, rect, one, searching),
        },
    );
}

/// One line of the form: a group's heading, or a row.
enum Line<'a> {
    Group(&'static str),
    Setting(&'a Row),
}

/// The rows, a heading wherever the group changes; none while a search mixes them.
fn lines(shown: &[Row], searching: bool) -> Vec<Line<'_>> {
    let mut out = Vec::new();
    let mut under = "";
    for one in shown {
        if !searching && one.group != under {
            under = one.group;
            out.push(Line::Group(under));
        }
        out.push(Line::Setting(one));
    }
    out
}

/// A group's heading, at the foot of its gap.
fn group(ctx: &mut Ctx, mut rect: Rect, name: &str) {
    let line = rect.take_bottom(ctx.tokens.row);
    Label::new(name, ctx.styles.label(Role::Faint)).draw(ctx, line);
}

/// The sign-in's terminal, which has the keys while it runs.
fn signing_in(ctx: &mut Ctx, pane: Rect, terminal: &Terminal) {
    ctx.quad(pane, ctx.styles.deep());
    ctx.hit(pane, Target::Login);
    let origin = (pane.x + ctx.tokens.sm, pane.y + ctx.tokens.sm);
    screen(ctx, pane, origin, (&terminal.screen(), true));
}

/// One row: its label, its group when a search mixes them, its value at the right.
fn setting(ctx: &mut Ctx, line: Rect, one: &Row, searching: bool) {
    hairline(ctx, line, ctx.styles.line());
    let mut room = line;
    let label = room.take_left(ctx.tokens.aside_mid);
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
            let word = if *on { "on" } else { "off" };
            let each = std::iter::once((word, *on, Target::SetPreference(*flip)));
            held_words(ctx, room, each, Role::Muted);
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

/// A skill's switch, its delete, then what it does; or the question its delete asks.
fn skill(
    ctx: &mut Ctx,
    mut room: Rect,
    (id, on, said): (&str, Option<bool>, &str),
    (deletes, asking): (bool, bool),
) {
    let (ground, sm) = (ctx.styles.ground(), ctx.tokens.sm);
    if asking {
        let question = Label::new("delete it? its file goes", ctx.styles.body(Role::Warn));
        question.left(ctx, &mut room, sm);
        let sure = Target::SkillDeleteSure(id.to_string());
        Button::new("yes, delete", sure, Role::Muted, ground).left(ctx, &mut room, sm);
        Button::new("keep", Target::SkillDeleteKeep, Role::Muted, ground).left(ctx, &mut room, sm);
        return;
    }
    match on {
        Some(on) => {
            let (word, role) = if on {
                ("on", Role::Text)
            } else {
                ("off", Role::Muted)
            };
            let flip = Target::SkillSwitch(id.to_string(), !on);
            Button::new(word, flip, role, ground).left(ctx, &mut room, sm);
        }
        None => {
            Label::new("always", ctx.styles.body(Role::Faint)).left(ctx, &mut room, sm);
        }
    }
    if deletes {
        let delete = Target::SkillDelete(id.to_string());
        Button::new("delete", delete, Role::Muted, ground).left(ctx, &mut room, sm);
    }
    Label::new(said, ctx.styles.small(Role::Faint)).draw(ctx, room);
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
        true => (ctx.styles.raised(), Role::Text),
        false => (ctx.styles.band(), Role::Muted),
    };
    ctx.quad(field, ground);
    ctx.hit(field, target.clone());
    let text = field.pad(Edges::across(sm, sm));
    Label::new(shown, ctx.styles.code(role)).draw(ctx, text);
}
