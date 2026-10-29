//! Text as the last frame drew it: the conversation's runs, the prompt's rows, and what a drag holds.

use groove_gfx::Rect;

/// One run of drawn text, and where each of its characters starts, its end last.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub rect: Rect,
    pub text: String,
    pub edges: Vec<f32>,
}

/// A run's characters from and to.
pub type Span = (usize, usize, usize);

/// The prompt's rows as drawn: each one's rect and piece of the buffer, and how far they scrolled.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Typed {
    pub rows: Vec<(Rect, (usize, usize, usize))>,
    pub scroll: f32,
}

impl Line {
    fn chars(&self) -> usize {
        self.edges.len().saturating_sub(1)
    }

    /// The character under `x`.
    fn under(&self, x: f32) -> usize {
        let x = x - self.rect.x;
        let past = self
            .edges
            .windows(2)
            .take_while(|pair| pair[1] <= x)
            .count();
        past.min(self.chars().saturating_sub(1))
    }

    /// The character boundary nearest `x`.
    fn column(&self, x: f32) -> usize {
        let x = x - self.rect.x;
        let half = |pair: &[f32]| (pair[0] + pair[1]) / 2.0;
        self.edges
            .windows(2)
            .take_while(|pair| half(pair) <= x)
            .count()
    }
}

/// Runs in reading order: row by row, then left to right.
pub fn ordered(mut lines: Vec<Line>) -> Vec<Line> {
    let key = |one: &Line| ((one.rect.y + one.rect.h / 2.0).round() as i64, one.rect.x);
    lines.sort_by(|a, b| {
        let (a, b) = (key(a), key(b));
        a.0.cmp(&b.0).then(a.1.total_cmp(&b.1))
    });
    lines
}

/// The place nearest a point: on its row, else the start of the next row, else the very end.
fn spot(lines: &[Line], (x, y): (f32, f32)) -> Option<(usize, usize)> {
    let on = |one: &Line| one.rect.y <= y && y < one.rect.bottom();
    let under = lines.iter().rposition(|one| on(one) && one.rect.x <= x);
    if let Some(at) = under.or_else(|| lines.iter().position(on)) {
        return Some((at, lines[at].column(x)));
    }
    if let Some(at) = lines.iter().position(|one| one.rect.y > y) {
        return Some((at, 0));
    }
    let last = lines.len().checked_sub(1)?;
    Some((last, lines[last].chars()))
}

/// The two ends of what a double click takes under a point: the word, or the spaces or marks
/// between words; a triple click takes the whole run.
pub fn around(lines: &[Line], point: (f32, f32), count: u32) -> Option<((f32, f32), (f32, f32))> {
    let (at, _) = spot(lines, point)?;
    let line = &lines[at];
    let chars: Vec<char> = line.text.chars().collect();
    let (start, end) = match count {
        2 => taken(&chars, line.under(point.0), kind),
        _ => (0, chars.len()),
    };
    let y = line.rect.y + line.rect.h / 2.0;
    let x = |column: usize| line.rect.x + line.edges[column];
    Some(((x(start), y), (x(end), y)))
}

/// The token under a point: the characters around it up to a space, without the marks that wrap it.
pub fn token(lines: &[Line], point: (f32, f32)) -> Option<String> {
    let (at, _) = spot(lines, point)?;
    let chars: Vec<char> = lines[at].text.chars().collect();
    let (start, end) = taken(&chars, lines[at].under(point.0), char::is_whitespace);
    let said: String = chars[start..end].iter().collect();
    let wraps = |one: char| "`'\"()[]<>{},;".contains(one);
    let said = said.trim_matches(wraps).trim_end_matches(['.', ':']);
    (!said.is_empty()).then(|| said.to_string())
}

/// The run of characters of one kind around `column`.
fn taken<K: PartialEq>(chars: &[char], column: usize, kind: impl Fn(char) -> K) -> (usize, usize) {
    let Some(&under) = chars.get(column) else {
        return (0, 0);
    };
    let of = kind(under);
    let start = chars[..column]
        .iter()
        .rposition(|one| kind(*one) != of)
        .map_or(0, |at| at + 1);
    let end = chars[column..]
        .iter()
        .position(|one| kind(*one) != of)
        .map_or(chars.len(), |at| column + at);
    (start, end)
}

/// A word's characters, the spaces, and the marks are three kinds.
fn kind(one: char) -> u8 {
    match one {
        one if one.is_alphanumeric() || one == '_' => 0,
        one if one.is_whitespace() => 1,
        _ => 2,
    }
}

/// What each run holds between two points.
pub fn held(lines: &[Line], (a, b): ((f32, f32), (f32, f32))) -> Vec<Span> {
    let (Some(a), Some(b)) = (spot(lines, a), spot(lines, b)) else {
        return Vec::new();
    };
    let (from, to) = (a.min(b), a.max(b));
    (from.0..=to.0)
        .map(|at| {
            let start = if at == from.0 { from.1 } else { 0 };
            let end = if at == to.0 { to.1 } else { lines[at].chars() };
            (at, start, end)
        })
        .filter(|(_, start, end)| start < end)
        .collect()
}

/// The text held: a space between runs of one row, a line break between rows.
pub fn text(lines: &[Line], spans: &[Span]) -> String {
    let mut out = String::new();
    let mut last: Option<Rect> = None;
    for &(at, start, end) in spans {
        let line = &lines[at];
        if let Some(prior) = last {
            let middle = line.rect.y + line.rect.h / 2.0;
            let same = prior.y <= middle && middle < prior.bottom();
            out.push(if same { ' ' } else { '\n' });
        }
        out.extend(line.text.chars().skip(start).take(end - start));
        last = Some(line.rect);
    }
    out
}
