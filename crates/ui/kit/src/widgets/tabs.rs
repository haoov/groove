use groove_gfx::{Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::text::row;
use crate::widgets::{Button, Text};

/// One tab: a rounded row on glass, held while selected and lit under the pointer.
pub struct Tab<'a, T> {
    pub label: &'a str,
    pub target: T,
    pub selected: bool,
    pub text: Text,
    /// The label's role while another tab is up.
    pub quiet: Role,
    pub tight: bool,
    pub close: Option<(Mark, T)>,
    pub italic: bool,
    /// The errors and warnings of the file it holds.
    pub tally: Option<(u32, u32)>,
    /// Its label in the middle of a box wider than it.
    pub centred: bool,
    pub mark: Option<Mark>,
}

impl<'a, T: Clone + PartialEq> Tab<'a, T> {
    pub fn new(label: &'a str, target: T, selected: bool) -> Self {
        Self {
            label,
            target,
            selected,
            text: Text::Label,
            quiet: Role::Muted,
            tight: false,
            close: None,
            italic: false,
            tally: None,
            centred: false,
            mark: None,
        }
    }

    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    pub fn text(mut self, text: Text) -> Self {
        self.text = text;
        self
    }

    pub fn quiet(mut self, role: Role) -> Self {
        self.quiet = role;
        self
    }

    /// The small padding, for tabs set in a header's end.
    pub fn tight(mut self) -> Self {
        self.tight = true;
        self
    }

    pub fn tally(mut self, tally: Option<(u32, u32)>) -> Self {
        self.tally = tally;
        self
    }

    pub fn close(mut self, mark: Mark, target: T) -> Self {
        self.close = Some((mark, target));
        self
    }

    /// Stands at the left of `room`, which gives up its box and `gap`.
    pub fn left<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, room: &mut Rect, gap: f32) -> Rect {
        let slot = crate::shape::stand(room, self.width(ctx), gap, crate::widgets::Side::Left);
        self.draw(ctx, slot);
        slot
    }

    /// Stands at the right of `room`, which gives up its box and `gap`.
    pub fn right<A: App<Target = T>>(
        self,
        ctx: &mut Ctx<'_, A>,
        room: &mut Rect,
        gap: f32,
    ) -> Rect {
        let slot = crate::shape::stand(room, self.width(ctx), gap, crate::widgets::Side::Right);
        self.draw(ctx, slot);
        slot
    }

    fn pad<A: App<Target = T>>(&self, ctx: &Ctx<'_, A>) -> f32 {
        if self.tight {
            ctx.tokens.sm
        } else {
            ctx.tokens.md
        }
    }

    fn style<A: App<Target = T>>(&self, ctx: &Ctx<'_, A>) -> TextStyle {
        let role = if self.selected {
            Role::Text
        } else {
            self.quiet
        };
        let style = self.text.style(&ctx.styles, role);
        ctx.styles.emphasised(style, false, self.italic)
    }

    fn width<A: App<Target = T>>(&self, ctx: &mut Ctx<'_, A>) -> f32 {
        let closing = match self.close {
            Some(_) => ctx.tokens.small + ctx.tokens.sm * 2.0,
            None => 0.0,
        };
        let tallied = self
            .tally
            .map_or(0.0, |tally| crate::widgets::faults_room(ctx, tally));
        let style = self.style(ctx);
        let lead = self.lead(ctx);
        ctx.measure(self.label, &style) + self.pad(ctx) * 2.0 + closing + tallied + lead
    }

    /// The room the mark takes before the label, its gap included.
    fn lead<A: App<Target = T>>(&self, ctx: &Ctx<'_, A>) -> f32 {
        match (self.mark, self.label.is_empty()) {
            (None, _) => 0.0,
            (Some(_), true) => ctx.tokens.icon,
            (Some(_), false) => ctx.tokens.icon + ctx.tokens.xs,
        }
    }

    fn draw<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, box_: Rect) {
        let tall = ctx.tokens.row.min(box_.h);
        let pill = crate::shape::band(box_, (box_.h - tall) / 2.0, tall);
        let on = ctx.interact(pill, self.target.clone());
        let ground = match (self.selected, on) {
            (true, _) => Some(ctx.chosen()),
            (false, true) => Some(ctx.styles.glass_hover()),
            (false, false) => None,
        };
        if let Some(color) = ground {
            ctx.rounded(pill, color, ctx.tokens.sm);
        }
        let (style, pad, lead) = (self.style(ctx), self.pad(ctx), self.lead(ctx));
        let mut end = pill;
        if let Some((mark, target)) = self.close {
            Button::icon(mark, 0, target, Role::Faint).right(ctx, &mut end, 0.0);
        }
        if let Some(tally) = self.tally {
            crate::widgets::faults(ctx, &mut end, tally);
        }
        let label =
            crate::text::elide(ctx, self.label, &style, (end.w - pad * 2.0 - lead).max(0.0));
        let pad = match self.centred {
            true => ((end.w - lead - ctx.measure(&label, &style)) / 2.0).max(pad),
            false => pad,
        };
        if let Some(mark) = self.mark {
            let at = crate::shape::box_in(pill, pill.x + pad, ctx.tokens.icon);
            ctx.icon(at, mark, 0, style.color);
        }
        row(ctx, pill, pad + lead, &label, style);
    }
}

/// Rounded tabs on glass from the left of `rect`, the selected one held; returns their boxes.
pub fn glass_tabs<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    tabs: &[(&str, A::Target)],
    selected: usize,
) -> Vec<Rect> {
    let marked: Vec<(Option<Mark>, &str, A::Target)> = tabs
        .iter()
        .map(|(label, target)| (None, *label, target.clone()))
        .collect();
    marked_glass_tabs(ctx, rect, &marked, selected)
}

/// The same, each tab with its mark before its label.
pub fn marked_glass_tabs<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    tabs: &[(Option<Mark>, &str, A::Target)],
    selected: usize,
) -> Vec<Rect> {
    let xs = ctx.tokens.xs;
    let mut room = rect.pad(groove_gfx::Edges::across(ctx.tokens.sm, 0.0));
    let made: Vec<Tab<'_, A::Target>> = tabs
        .iter()
        .enumerate()
        .map(|(i, (mark, label, target))| {
            let mut tab = Tab::new(label, target.clone(), i == selected);
            tab.centred = true;
            tab.mark = *mark;
            tab
        })
        .collect();
    let natural: Vec<f32> = made.iter().map(|tab| tab.width(ctx)).collect();
    let free = (room.w - xs * (natural.len().max(1) - 1) as f32).max(0.0);
    let wide = shares(&natural, free);
    made.into_iter()
        .zip(wide)
        .map(|(tab, wide)| {
            let slot = crate::shape::from_left(&mut room, wide, xs);
            tab.draw(ctx, slot);
            slot
        })
        .collect()
}

/// Each width grown by an even share of the room left, or shrunk in proportion when none is.
fn shares(natural: &[f32], room: f32) -> Vec<f32> {
    let total: f32 = natural.iter().sum();
    if total <= room {
        let extra = (room - total) / natural.len().max(1) as f32;
        return natural.iter().map(|one| one + extra).collect();
    }
    natural
        .iter()
        .map(|one| one * room / total.max(1.0))
        .collect()
}
