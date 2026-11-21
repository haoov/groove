//! Signals firing across the hex lattice while an agent works: each leaves a corner anywhere on
//! the ground, runs the lattice's edges down, fires the corner it reaches and branches on, the path
//! it crossed glowing behind it as it fades. The clock alone draws them, so a frame at any tick
//! shows the same.

use groove_gfx::{Color, Rect, Shape};

use crate::base::ctx::{App, Ctx};
use crate::base::tokens::{HEX, NEURON};

/// How often a frame is drawn while they fire, in ms.
pub const NEURON_MS: u64 = 40;
/// A cascade's life, how far apart the cascades start, and how long one hop takes, in ms.
const PERIOD: u64 = 4200;
const STAGGER: u64 = 1400;
const HOP: u64 = 588;
/// How long a crossed edge glows, a fired corner shines, and its halo spreads, in ms.
const GLOW: u64 = 1400;
const SHINE: u64 = 1600;
const SPREAD: u64 = 1000;
const CASCADES: u64 = 3;
/// How many hops a cascade runs, and how many signals one level holds at most.
const LEVELS: u64 = 4;
const WIDEST: usize = 4;
/// The strongest a glow and a halo stand, dark then light: a dark hue blended in linear light
/// over a pale ground reads far weaker than a light one over a dark ground.
const GLOW_ALPHA: (u8, u8) = (120, 230);
const HALO_ALPHA: (u8, u8) = (80, 170);

/// A corner of the lattice: `j` half hex widths across, `l` rows of corners down.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Corner {
    j: i32,
    l: i32,
}

/// One hop of a signal: the corners it runs through, from when, in ms of its cascade.
struct Hop {
    path: Vec<(f32, f32)>,
    from: u64,
}

/// The lattice inside a ground: its hexes' corner radius and the corners a signal may take.
struct Lattice {
    s: f32,
    across: (i32, i32),
    down: (i32, i32),
}

impl Lattice {
    fn of(rect: Rect, s: f32) -> Self {
        let (half, row) = (half_width(s), s * f32::from(3u8) / 2.0);
        Self {
            s,
            across: (
                (rect.x / half).ceil() as i32 + 1,
                (rect.right() / half) as i32 - 1,
            ),
            down: (
                (rect.y / row).ceil() as i32,
                (rect.bottom() / row) as i32 - 1,
            ),
        }
    }

    /// Where a corner stands in the window: a hex's top and bottom corners alternate along a row.
    fn at(&self, c: Corner) -> (f32, f32) {
        let low = (c.j + c.l).rem_euclid(2) == 0;
        let y = (3 * c.l) as f32 * self.s / 2.0 + if low { self.s / 2.0 } else { 0.0 };
        (c.j as f32 * half_width(self.s), y)
    }

    fn row(&self, l: i32) -> bool {
        (self.down.0..=self.down.1).contains(&l)
    }

    fn holds(&self) -> bool {
        self.across.1 > self.across.0 && self.down.1 > self.down.0
    }
}

fn half_width(s: f32) -> f32 {
    s * f32::from(3u8).sqrt() / 2.0
}

/// A splitmix stream: the same seed, the same cascade.
struct Dice(u64);

impl Dice {
    fn roll(&mut self, sides: u64) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ (z >> 31)) % sides.max(1)
    }

    fn side(&mut self) -> i32 {
        if self.roll(2) == 0 { -1 } else { 1 }
    }
}

/// The signals over `rect` at this tick.
pub fn neurons<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    let lattice = Lattice::of(rect, (HEX * ctx.tokens.scale).round());
    if !lattice.holds() {
        return;
    }
    for cascade in 0..CASCADES {
        let Some(since) = ctx.tick.checked_sub(cascade * STAGGER) else {
            continue;
        };
        let mut dice = Dice(cascade * 7919 + since / PERIOD * 104_729 + 1);
        let (hops, fired) = grown(&lattice, &mut dice);
        let now = since % PERIOD;
        for hop in hops.iter().filter(|hop| hop.from <= now) {
            glow(ctx, &lattice, hop, now - hop.from);
        }
        for &(corner, at) in fired.iter().filter(|(_, at)| *at <= now) {
            fire(ctx, lattice.at(corner), now - at);
        }
    }
}

/// A cascade from a corner anywhere on the ground, heading down: each level's signals branch,
/// run on or die out.
fn grown(lattice: &Lattice, dice: &mut Dice) -> (Vec<Hop>, Vec<(Corner, u64)>) {
    let ((left, right), (top, bottom)) = (lattice.across, lattice.down);
    let root = Corner {
        j: left + dice.roll((right - left) as u64) as i32,
        l: top + dice.roll((bottom - top) as u64) as i32,
    };
    let (mut hops, mut fired) = (Vec::new(), vec![(root, 0)]);
    let mut front = vec![root];
    for level in 0..LEVELS {
        let mut next = Vec::new();
        for from in front {
            let kids = if level == 0 {
                2
            } else {
                [0, 1, 1, 2, 2][dice.roll(5) as usize]
            };
            for kid in 0..kids {
                if next.len() >= WIDEST || !lattice.row(from.l + 1) {
                    break;
                }
                let dir = if kids == 2 { kid * 2 - 1 } else { dice.side() };
                let (path, to) = walk(lattice, dice, from, dir);
                hops.push(Hop {
                    path,
                    from: level * HOP,
                });
                fired.push((to, (level + 1) * HOP));
                next.push(to);
            }
        }
        front = next;
    }
    (hops, fired)
}

/// One hop: along a row's edges toward `dir`, then down a hex's side, once or twice.
fn walk(lattice: &Lattice, dice: &mut Dice, from: Corner, dir: i32) -> (Vec<(f32, f32)>, Corner) {
    let (left, right) = lattice.across;
    let mut at = from;
    let mut path = vec![lattice.at(at)];
    let step = |at: &mut Corner, path: &mut Vec<(f32, f32)>| {
        let j = at.j + dir;
        at.j = if (left..=right).contains(&j) {
            j
        } else {
            at.j - dir
        };
        path.push(lattice.at(*at));
    };
    for drop in 0..=dice.roll(2) {
        if !lattice.row(at.l + 1) {
            break;
        }
        for _ in 0..dice.roll(3) + u64::from(drop == 0) {
            step(&mut at, &mut path);
        }
        // A side runs down from a hex's bottom corner.
        if (at.j + at.l).rem_euclid(2) != 0 {
            step(&mut at, &mut path);
        }
        at.l += 1;
        path.push(lattice.at(at));
    }
    (path, at)
}

/// A hop `age` ms after it left: the lattice lit along what it crossed, brightest at its head.
fn glow<A: App>(ctx: &mut Ctx<'_, A>, lattice: &Lattice, hop: &Hop, age: u64) {
    let length: f32 = hop.path.windows(2).map(|w| distance(w[0], w[1])).sum();
    if length <= 0.0 {
        return;
    }
    let ran = age.min(HOP) as f32 / HOP as f32 * length;
    let step = lattice.s / f32::from(3u8);
    let mut x = 0.0;
    while x <= ran {
        let passed = (x / length * HOP as f32) as u64;
        let since = age.saturating_sub(passed);
        let head = ran - x < step && age < HOP;
        let left = 1.0 - since as f32 / GLOW as f32;
        let strength = if head { 1.0 } else { left.max(0.0).powi(2) };
        if strength > 0.0 {
            let color = ctx.styles.neuron();
            lit(
                ctx,
                along(&hop.path, x),
                color.with_alpha(alpha(strength, ctx.styles.theme_alpha(GLOW_ALPHA))),
            );
        }
        x += step;
    }
}

/// The lattice's edges near `at`, fading out from it.
fn lit<A: App>(ctx: &mut Ctx<'_, A>, (x, y): (f32, f32), color: Color) {
    let s = (HEX * ctx.tokens.scale).round();
    let reach = s / 2.0;
    let shape = Shape {
        lattice: s,
        stroke: ctx.tokens.hairline * 2.0,
        radius: reach,
        blur: reach,
        ..Shape::default()
    };
    let rect = Rect::new(x - reach, y - reach, reach * 2.0, reach * 2.0);
    ctx.under_shaped(rect, color, shape);
}

/// A corner `age` ms after a signal reached it: its body shining, its halo spreading out.
fn fire<A: App>(ctx: &mut Ctx<'_, A>, (x, y): (f32, f32), age: u64) {
    let (body, halo) = (NEURON.0 * ctx.tokens.scale, NEURON.1 * ctx.tokens.scale);
    let color = ctx.styles.neuron();
    let shine = (1.0 - age as f32 / SHINE as f32).max(0.0).powi(2);
    if shine > 0.0 {
        let dot = Rect::new(x - body, y - body, body * 2.0, body * 2.0);
        let shape = Shape {
            radius: body,
            ..Shape::default()
        };
        ctx.under_shaped(dot, color.with_alpha(alpha(shine, u8::MAX)), shape);
    }
    if age < SPREAD {
        let spread = age as f32 / SPREAD as f32;
        let r = halo * (1.0 + spread) / 2.0;
        let ring = Rect::new(x - r, y - r, r * 2.0, r * 2.0);
        let shape = Shape {
            radius: r,
            blur: r / 2.0,
            ..Shape::default()
        };
        let faded = alpha(1.0 - spread, ctx.styles.theme_alpha(HALO_ALPHA));
        ctx.under_shaped(ring, color.with_alpha(faded), shape);
    }
}

fn alpha(strength: f32, most: u8) -> u8 {
    (strength.clamp(0.0, 1.0) * f32::from(most)) as u8
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

/// The point `x` pixels along `path`.
fn along(path: &[(f32, f32)], mut x: f32) -> (f32, f32) {
    for w in path.windows(2) {
        let d = distance(w[0], w[1]);
        if x <= d && d > 0.0 {
            let t = x / d;
            return (
                w[0].0 + (w[1].0 - w[0].0) * t,
                w[0].1 + (w[1].1 - w[0].1) * t,
            );
        }
        x -= d;
    }
    path.last().copied().unwrap_or_default()
}

#[cfg(test)]
mod tests;
