//! The Agent section's own values: a skill's switch and delete, a routine's switch and run.

use groove_gfx::Rect;

use super::super::rows::Value;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::Button;

/// A routine's switch, its run and what it does; or one trigger's switch.
pub(super) fn routine(ctx: &mut Ctx, mut room: Rect, value: &Value) {
    let (ground, sm) = (ctx.styles.ground(), ctx.tokens.sm);
    let switch = |on: bool, target: Target| {
        let (word, role) = if on {
            ("on", Role::Text)
        } else {
            ("off", Role::Muted)
        };
        Button::new(word, target, role, ground)
    };
    match value {
        Value::Switch { on, target } => {
            switch(*on, target.clone()).left(ctx, &mut room, sm);
        }
        Value::Routine { id, on, said, runs } => {
            let flip = match on {
                true => Target::RoutineOff(id.clone()),
                false => Target::RoutineOn(id.clone()),
            };
            switch(*on, flip).left(ctx, &mut room, sm);
            if *runs {
                let run = Target::RoutineRun(id.clone());
                Button::new("run", run, Role::Muted, ground).left(ctx, &mut room, sm);
            }
            Label::new(said, ctx.styles.small(Role::Faint)).draw(ctx, room);
        }
        _ => {}
    }
}

/// A skill's switch, its delete, then what it does; or the question its delete asks.
pub(super) fn skill(
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
