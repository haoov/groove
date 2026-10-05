//! Boxes and text leaves, laid out by Taffy. A view describes them here and never names Taffy.

use groove_gfx::{Edges, Rect, TextStyle};
use taffy::{
    AlignItems, AvailableSpace, Dimension, FlexDirection, LengthPercentage, NodeId, Style,
    TaffyTree, compute_leaf_layout,
};

use crate::base::ctx::{App, Ctx};

mod measure;
#[cfg(test)]
mod tests;

use measure::{Text, measure};

/// A box or a leaf of the layout being described.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node(NodeId);

/// How a box sits in its parent and spaces its children.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Spec {
    gap: f32,
    pad: Edges,
    grow: f32,
    width: Option<f32>,
    height: Option<f32>,
    centred: bool,
}

impl Spec {
    /// Space between the children.
    pub fn gap(self, gap: f32) -> Self {
        Self { gap, ..self }
    }

    /// Space between the box's edges and its children.
    pub fn pad(self, pad: Edges) -> Self {
        Self { pad, ..self }
    }

    /// The share of the parent's spare room the box takes.
    pub fn grow(self, grow: f32) -> Self {
        Self { grow, ..self }
    }

    /// A width the box keeps, however little room its parent has.
    pub fn width(self, width: f32) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    /// A height the box keeps, however little room its parent has.
    pub fn height(self, height: f32) -> Self {
        Self {
            height: Some(height),
            ..self
        }
    }

    /// The children stand in the middle of the cross axis instead of stretching across it.
    pub fn centred(self) -> Self {
        Self {
            centred: true,
            ..self
        }
    }

    fn style(self, direction: FlexDirection) -> Style {
        let length = LengthPercentage::length;
        let size = |side: Option<f32>| side.map_or(Dimension::auto(), Dimension::length);
        Style {
            flex_direction: direction,
            gap: taffy::Size {
                width: length(self.gap),
                height: length(self.gap),
            },
            padding: taffy::Rect {
                left: length(self.pad.left),
                right: length(self.pad.right),
                top: length(self.pad.top),
                bottom: length(self.pad.bottom),
            },
            flex_grow: self.grow,
            flex_shrink: match self.width.or(self.height) {
                Some(_) => 0.0,
                None => 1.0,
            },
            size: taffy::Size {
                width: size(self.width),
                height: size(self.height),
            },
            align_items: self.centred.then_some(AlignItems::CENTER),
            ..Style::default()
        }
    }
}

/// One frame's layout: describe the boxes, solve them in a rect, then read each one's rect.
pub struct Boxes {
    tree: TaffyTree<Text>,
    origin: (f32, f32),
}

impl Default for Boxes {
    fn default() -> Self {
        Self::new()
    }
}

impl Boxes {
    pub fn new() -> Self {
        let mut tree = TaffyTree::with_capacity(32);
        tree.disable_rounding();
        Self {
            tree,
            origin: (0.0, 0.0),
        }
    }

    /// A box laying its children out left to right.
    pub fn row(&mut self, spec: Spec, children: &[Node]) -> Node {
        self.parent(spec.style(FlexDirection::Row), children)
    }

    /// A box laying its children out top to bottom.
    pub fn column(&mut self, spec: Spec, children: &[Node]) -> Node {
        self.parent(spec.style(FlexDirection::Column), children)
    }

    /// An empty box the view fills itself, such as a list or a grid of cells.
    pub fn leaf(&mut self, spec: Spec) -> Node {
        let id = self.tree.new_leaf(yielding(spec.style(FlexDirection::Row)));
        Node(id.unwrap_or_else(|_| dead()))
    }

    /// Words wrapped to the width the box is given, each row `line` tall.
    pub fn text(&mut self, spec: Spec, text: &str, style: TextStyle, line: f32) -> Node {
        let held = Text::new(text, style, line);
        let id = self
            .tree
            .new_leaf_with_context(spec.style(FlexDirection::Row), held);
        Node(id.unwrap_or_else(|_| dead()))
    }

    /// Lays `root` out to fill `rect`, measuring every text leaf with the frame's fonts.
    pub fn solve<A: App>(&mut self, ctx: &mut Ctx<'_, A>, root: Node, rect: Rect) {
        self.compute(root, rect, |held, known, room| match held {
            Some(text) => measure(ctx, text, known, room),
            None => taffy::Size::ZERO,
        });
    }

    /// Lays out a tree of boxes alone, where no frame is drawing. A text leaf stands empty.
    pub fn place(&mut self, root: Node, rect: Rect) {
        self.compute(root, rect, |_, _, _| taffy::Size::ZERO);
    }

    fn compute(
        &mut self,
        root: Node,
        rect: Rect,
        mut size: impl FnMut(
            Option<&mut Text>,
            taffy::Size<Option<f32>>,
            taffy::Size<AvailableSpace>,
        ) -> taffy::Size<f32>,
    ) {
        self.origin = (rect.x, rect.y);
        if let Ok(style) = self.tree.style(root.0) {
            let filled = Style {
                size: taffy::Size {
                    width: Dimension::length(rect.w),
                    height: Dimension::length(rect.h),
                },
                ..style.clone()
            };
            let _ = self.tree.set_style(root.0, filled);
        }
        let room = taffy::Size {
            width: AvailableSpace::Definite(rect.w),
            height: AvailableSpace::Definite(rect.h),
        };
        let _ = self
            .tree
            .compute_layout_with_measure(root.0, room, |input, _, held, style| {
                compute_leaf_layout(
                    input,
                    style,
                    |_, basis| basis,
                    |known, room| size(held, known, room),
                )
            });
    }

    /// Where `node` landed in the window. Empty before `solve`.
    pub fn rect(&self, node: Node) -> Rect {
        let Ok(own) = self.tree.layout(node.0) else {
            return Rect::default();
        };
        let (mut x, mut y) = (own.location.x, own.location.y);
        let mut at = node.0;
        while let Some(parent) = self.tree.parent(at) {
            if let Ok(layout) = self.tree.layout(parent) {
                x += layout.location.x;
                y += layout.location.y;
            }
            at = parent;
        }
        Rect::new(
            self.origin.0 + x,
            self.origin.1 + y,
            own.size.width,
            own.size.height,
        )
    }

    fn parent(&mut self, style: Style, children: &[Node]) -> Node {
        let ids: Vec<NodeId> = children.iter().map(|child| child.0).collect();
        let id = self.tree.new_with_children(yielding(style), &ids);
        Node(id.unwrap_or_else(|_| dead()))
    }
}

/// The box may shrink to nothing, not only to its content's narrowest.
fn yielding(style: Style) -> Style {
    Style {
        min_size: taffy::Size {
            width: taffy::LengthPercentageAuto::length(0.0),
            height: taffy::LengthPercentageAuto::length(0.0),
        },
        ..style
    }
}

/// A node the tree does not hold; it lays out empty. Taffy only fails on a missing child.
fn dead() -> NodeId {
    NodeId::from(u64::MAX)
}
