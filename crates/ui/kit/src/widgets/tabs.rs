use std::borrow::Cow;

use groove_gfx::{Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::text::row;
use crate::widgets::{Button, Text};

/// One tab: a segment of a hairline box, its label small and mono, uppercased unless it is a name;
/// held while selected and lit under the pointer, square. Alone it takes the box to itself; a
/// set laid by `tab_group` shares one.
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
    /// The label as given: a name, never uppercased.
    pub cased: bool,
}

impl<'a, T: Clone + PartialEq> Tab<'a, T> {
    pub fn new(label: &'a str, target: T, selected: bool) -> Self {
        Self {
            label,
            target,
            selected,
            text: Text::Label,
            quiet: Role::Faint,
            tight: false,
            close: None,
            italic: false,
            tally: None,
            centred: false,
            mark: None,
            cased: false,
        }
    }

    /// The label as given, for a name: a file, a terminal, an object, a container.
    pub fn keep_case(mut self) -> Self {
        self.cased = true;
        self
    }

    /// The label as it reads: a word in capitals, a name as given.
    fn shown(&self) -> Cow<'a, str> {
        match self.cased {
            true => Cow::Borrowed(self.label),
            false => Cow::Owned(self.label.to_uppercase()),
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
        self.draw(ctx, slot, true);
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
        self.draw(ctx, slot, true);
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
        ctx.styles
            .emphasised(ctx.styles.tab(role), false, self.italic)
    }

    pub fn width<A: App<Target = T>>(&self, ctx: &mut Ctx<'_, A>) -> f32 {
        let closing = match self.close {
            Some(_) => ctx.tokens.small + ctx.tokens.sm * 2.0,
            None => 0.0,
        };
        let tallied = self
            .tally
            .map_or(0.0, |tally| crate::widgets::faults_room(ctx, tally));
        let style = self.style(ctx);
        let lead = self.lead(ctx);
        ctx.measure(&self.shown(), &style) + self.pad(ctx) * 2.0 + closing + tallied + lead
    }

    /// The room the mark takes before the label, its gap included.
    fn lead<A: App<Target = T>>(&self, ctx: &Ctx<'_, A>) -> f32 {
        match (self.mark, self.label.is_empty()) {
            (None, _) => 0.0,
            (Some(_), true) => ctx.tokens.icon,
            (Some(_), false) => ctx.tokens.icon + ctx.tokens.xs,
        }
    }

    /// The tab in `box_`, its box edged when it stands alone.
    fn draw<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, box_: Rect, edged: bool) {
        let pill = segment(ctx, box_);
        let on = ctx.interact(pill, self.target.clone());
        let ground = match (self.selected, on) {
            (true, _) => Some(ctx.chosen()),
            (false, true) => Some(ctx.styles.glass_hover()),
            (false, false) => None,
        };
        if let Some(color) = ground {
            ctx.quad(pill, color);
        }
        if edged {
            edge(ctx, pill);
        }
        let (style, pad, lead) = (self.style(ctx), self.pad(ctx), self.lead(ctx));
        let shown = self.shown();
        let mut end = pill;
        if let Some((mark, target)) = self.close {
            Button::icon(mark, 0, target, Role::Faint).right(ctx, &mut end, 0.0);
        }
        if let Some(tally) = self.tally {
            crate::widgets::faults(ctx, &mut end, tally);
        }
        let label = crate::text::elide(ctx, &shown, &style, (end.w - pad * 2.0 - lead).max(0.0));
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

/// The same, each tab with its mark before its label, the set filling `rect` in one box.
pub fn marked_glass_tabs<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    tabs: &[(Option<Mark>, &str, A::Target)],
    selected: usize,
) -> Vec<Rect> {
    let mut room = rect.pad(groove_gfx::Edges::across(ctx.tokens.sm, ctx.tokens.sm));
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
    let wide = shares(&natural, room.w.max(0.0));
    let slots: Vec<Rect> = wide
        .iter()
        .map(|wide| crate::shape::from_left(&mut room, *wide, 0.0))
        .collect();
    for (tab, slot) in made.into_iter().zip(&slots) {
        tab.draw(ctx, *slot, false);
    }
    if let (Some(first), Some(last)) = (slots.first(), slots.last()) {
        let whole = Rect::new(first.x, first.y, last.right() - first.x, first.h);
        edge(ctx, segment(ctx, whole));
    }
    slots
}

/// A set of tabs edge to edge in one hairline box, standing at `side` of `room`, which gives up
/// the box and `gap`. Returns the box.
pub fn tab_group<A: App>(
    ctx: &mut Ctx<'_, A>,
    room: &mut Rect,
    tabs: Vec<Tab<'_, A::Target>>,
    (side, gap): (crate::widgets::Side, f32),
) -> Rect {
    let wide: Vec<f32> = tabs.iter().map(|tab| tab.width(ctx)).collect();
    let total: f32 = wide.iter().sum();
    let whole = crate::shape::stand(room, total, gap, side);
    let mut lay = whole;
    for (tab, wide) in tabs.into_iter().zip(wide) {
        let slot = crate::shape::from_left(&mut lay, wide, 0.0);
        tab.draw(ctx, slot, false);
    }
    if !whole.is_empty() {
        edge(ctx, segment(ctx, whole));
    }
    whole
}

/// How wide a set of tabs stands as one group.
pub fn group_width<A: App>(ctx: &mut Ctx<'_, A>, tabs: &[Tab<'_, A::Target>]) -> f32 {
    tabs.iter().map(|tab| tab.width(ctx)).sum()
}

/// The band a tab stands in inside its box: a little under a row, in the middle.
fn segment<A: App>(ctx: &Ctx<'_, A>, box_: Rect) -> Rect {
    let tall = (ctx.tokens.row - ctx.tokens.xs).min(box_.h);
    crate::shape::band(box_, (box_.h - tall) / 2.0, tall)
}

/// The hairline box around a tab or a set of them.
fn edge<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    let color = ctx.styles.line();
    ctx.ring(rect, color, ctx.tokens.round, ctx.tokens.hairline);
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
