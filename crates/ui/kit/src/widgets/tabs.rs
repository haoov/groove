use groove_gfx::{Color, Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::{hairline, hoverable};
use crate::text::row;
use crate::widgets::{Button, Text};

/// One tab: its label, underlined in the accent while selected, and the mark that closes it.
pub struct Tab<'a, T> {
    pub label: &'a str,
    pub target: T,
    pub selected: bool,
    pub text: Text,
    /// The label's role while another tab is up.
    pub quiet: Role,
    /// The selected tab's ground in place of the accent underline.
    pub ground: Option<Color>,
    pub tight: bool,
    pub close: Option<(Mark, T)>,
    pub italic: bool,
    /// The errors and warnings of the file it holds.
    pub tally: Option<(u32, u32)>,
    /// A rounded row on glass, held while selected and lit under the pointer.
    pub glass: bool,
    /// Its label in the middle of a box wider than it.
    pub centred: bool,
}

impl<'a, T: Clone + PartialEq> Tab<'a, T> {
    pub fn new(label: &'a str, target: T, selected: bool) -> Self {
        Self {
            label,
            target,
            selected,
            text: Text::Label,
            quiet: Role::Faint,
            ground: None,
            tight: false,
            close: None,
            italic: false,
            tally: None,
            glass: false,
            centred: false,
        }
    }

    pub fn glass(mut self) -> Self {
        self.glass = true;
        self
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

    pub fn ground(mut self, ground: Color) -> Self {
        self.ground = Some(ground);
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
        if self.glass {
            let style = match self.selected {
                true => ctx.styles.label(Role::Text),
                false => ctx.styles.label(Role::Muted),
            };
            return ctx.styles.emphasised(style, false, self.italic);
        }
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
        ctx.measure(self.label, &style) + self.pad(ctx) * 2.0 + closing + tallied
    }

    /// The label is drawn whole: the tab is as wide as it.
    fn draw<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, box_: Rect) {
        if self.glass {
            return self.drawn_on_glass(ctx, box_);
        }
        hoverable(ctx, box_, self.target.clone());
        match (self.selected, self.ground) {
            (true, Some(ground)) => ctx.quad(box_, ground),
            (true, None) => {
                crate::shape::ground(ctx, box_, crate::base::ground::Ground::Held);
                let thick = ctx.tokens.hairline * 2.0;
                let under = Rect::new(box_.x, box_.bottom() - thick, box_.w, thick);
                ctx.quad(under, ctx.styles.chosen());
            }
            (false, _) => {}
        }
        let style = self.style(ctx);
        let pad = self.pad(ctx);
        let mut end = box_;
        if let Some((mark, target)) = self.close {
            Button::icon(mark, 0, target, Role::Faint).right(ctx, &mut end, 0.0);
        }
        if let Some(tally) = self.tally {
            crate::widgets::faults(ctx, &mut end, tally);
        }
        row(ctx, box_, pad, self.label, style);
    }

    fn drawn_on_glass<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, box_: Rect) {
        let tall = ctx.tokens.row.min(box_.h);
        let pill = crate::shape::band(box_, (box_.h - tall) / 2.0, tall);
        let on = ctx.interact(pill, self.target.clone());
        let ground = match (self.selected, on) {
            (true, _) => Some(ctx.styles.glass_chosen()),
            (false, true) => Some(ctx.styles.glass_hover()),
            (false, false) => None,
        };
        if let Some(color) = ground {
            ctx.rounded(pill, color, ctx.tokens.sm);
        }
        let (style, pad) = (self.style(ctx), self.pad(ctx));
        let mut end = pill;
        if let Some((mark, target)) = self.close {
            Button::icon(mark, 0, target, Role::Faint).right(ctx, &mut end, 0.0);
        }
        if let Some(tally) = self.tally {
            crate::widgets::faults(ctx, &mut end, tally);
        }
        let label = crate::text::elide(ctx, self.label, &style, (end.w - pad * 2.0).max(0.0));
        let pad = match self.centred {
            true => ((end.w - ctx.measure(&label, &style)) / 2.0).max(pad),
            false => pad,
        };
        row(ctx, pill, pad, &label, style);
    }
}

/// Rounded tabs on glass from the left of `rect`, the selected one held; returns their boxes.
pub fn glass_tabs<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    tabs: &[(&str, A::Target)],
    selected: usize,
) -> Vec<Rect> {
    let xs = ctx.tokens.xs;
    let mut room = rect.pad(groove_gfx::Edges::across(ctx.tokens.sm, 0.0));
    let made: Vec<Tab<'_, A::Target>> = tabs
        .iter()
        .enumerate()
        .map(|(i, (label, target))| {
            let mut tab = Tab::new(label, target.clone(), i == selected).glass();
            tab.centred = true;
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

/// Tabs from the left of `rect`, the selected one underlined, a hairline under; returns their boxes.
pub fn tabs<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    tabs: &[(&str, A::Target)],
    selected: usize,
) -> Vec<Rect> {
    let line = ctx.styles.line();
    hairline(ctx, rect, line);
    let mut room = rect;
    tabs.iter()
        .enumerate()
        .map(|(i, (label, target))| {
            Tab::new(label, target.clone(), i == selected).left(ctx, &mut room, 0.0)
        })
        .collect()
}
