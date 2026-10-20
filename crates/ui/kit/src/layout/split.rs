//! A rect split into boxes top to bottom or left to right, one for each spec.

use groove_gfx::Rect;

use super::{Boxes, Node, Spec};

/// `rect` laid out top to bottom, one box for each spec.
pub fn column_in<const N: usize>(rect: Rect, specs: [Spec; N]) -> [Rect; N] {
    split(rect, specs, Boxes::column)
}

/// `rect` laid out top to bottom, one box for each of as many specs as the view has.
pub fn column_of(rect: Rect, specs: &[Spec]) -> Vec<Rect> {
    let mut boxes = Boxes::new();
    let nodes: Vec<Node> = specs.iter().map(|spec| boxes.leaf(*spec)).collect();
    let root = boxes.column(Spec::default(), &nodes);
    boxes.place(root, rect);
    nodes.iter().map(|node| boxes.rect(*node)).collect()
}

/// `rect` laid out top to bottom from `offset` above its top, one box for each spec.
pub fn column_from(rect: Rect, offset: f32, specs: &[Spec]) -> Vec<Rect> {
    let moved = Rect {
        y: rect.y - offset,
        ..rect
    };
    column_of(moved, specs)
}

/// `rect` laid out left to right, one box for each spec.
pub fn row_in<const N: usize>(rect: Rect, specs: [Spec; N]) -> [Rect; N] {
    split(rect, specs, Boxes::row)
}

fn split<const N: usize>(
    rect: Rect,
    specs: [Spec; N],
    parent: fn(&mut Boxes, Spec, &[Node]) -> Node,
) -> [Rect; N] {
    let mut boxes = Boxes::new();
    let nodes = specs.map(|spec| boxes.leaf(spec));
    let root = parent(&mut boxes, Spec::default(), &nodes);
    boxes.place(root, rect);
    nodes.map(|node| boxes.rect(node))
}
