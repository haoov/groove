//! The workspace's tabs, and the face the code surface takes on each.

use groove_types::DiffView;

/// Which tab of the workspace is up.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Overview,
    /// The whole change as one stream.
    Diff,
    /// The open files, one tab each, edited here.
    Files,
}

impl Default for Face {
    fn default() -> Self {
        Face::Stream(DiffView::default())
    }
}

impl Tab {
    /// Every tab, in the order the strip shows them.
    pub const ALL: [Tab; 3] = [Tab::Overview, Tab::Diff, Tab::Files];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::Diff => "diff",
            Tab::Files => "files",
        }
    }

    /// Whether the tab brings its own list beside the workspace.
    pub fn has_sidebar(self) -> bool {
        match self {
            Tab::Overview => false,
            Tab::Diff | Tab::Files => true,
        }
    }
}

/// What the code surface draws: the active file, or the stream in one of its views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    File,
    Stream(DiffView),
}
