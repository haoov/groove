//! The opaque panels: each headed by its band, ruled off under it; a folded one is its band alone.

use groove_gfx::{Color, Fonts, Frame, Rect};
use groove_ui_kit::base::style::Styles;
use groove_ui_kit::base::tokens::Tokens;

use crate::hit::Target;
use crate::layout::Layout;
use crate::tests::{full_app, window};
use crate::views::session::Tab;
use crate::{Ui, view};

/// Each quad of the frame in `color`, with its layer and its place in that layer.
fn quads(frame: &Frame, color: Color) -> Vec<(usize, usize, Rect)> {
    let layers = frame.layers().iter().enumerate();
    let all =
        layers.flat_map(|(at, one)| one.quads.iter().enumerate().map(move |(i, q)| (at, i, q)));
    all.filter(|(_, _, quad)| quad.color == color)
        .map(|(at, i, quad)| (at, i, quad.rect))
        .collect()
}

#[test]
fn every_open_panel_is_headed_by_its_band_ruled_off_under_it() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.manual = true;
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let layout = Layout::of(window(), &ui);
    let styles = Styles::new(app.config.theme(), Tokens::new(1.0));
    let bands = quads(&frame, styles.panel_head());
    let rules = quads(&frame, styles.line());
    let bar = Tokens::new(1.0).bar;
    for (name, panel) in [
        ("sessions", layout.sessions),
        ("feed", layout.feed),
        ("strip", layout.strip),
        ("drawer", layout.manual),
    ] {
        let top = |rect: &Rect| {
            rect.y <= panel.y + 1.0
                && rect.x <= panel.x + 1.0
                && rect.right() >= panel.right() - 1.0
        };
        assert!(
            bands.iter().any(|(_, _, one)| top(one)),
            "{name}: its band at its top"
        );
        let under =
            |rect: &Rect| (rect.bottom() - (panel.y + bar)).abs() <= 1.0 && rect.w >= panel.w - 1.0;
        assert!(
            rules.iter().any(|(_, _, one)| under(one)),
            "{name}: ruled off under its band"
        );
    }
}

#[test]
fn a_folded_capsule_is_its_band_and_what_it_holds_stands_on_it() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.stowed = true;
    ui.session.tab = Tab::Diff;
    ui.hover = Some(Target::Tab(Tab::Files));
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let layout = Layout::of(window(), &ui);
    let styles = Styles::new(app.config.theme(), Tokens::new(1.0));
    let band = quads(&frame, styles.panel_head())
        .into_iter()
        .find(|(_, _, one)| {
            (one.y - layout.header.y).abs() <= 1.0
                && (one.right() - layout.header.right()).abs() <= 1.0
        });
    let (layer, at, capsule) = band.expect("the capsule's band");
    let files = hits.rect_of(&Target::Tab(Tab::Files)).expect("a tab");
    assert!(
        capsule.x <= files.x && capsule.right() >= files.right(),
        "around its tabs"
    );
    let lit = quads(&frame, styles.glass_hover())
        .into_iter()
        .find(|(_, _, one)| *one == files);
    let (lit_layer, lit_at, _) = lit.expect("the tab under the pointer is lit");
    assert!(
        (lit_layer, lit_at) > (layer, at),
        "lit over the band, not under it"
    );
}

#[test]
fn a_set_of_tabs_stands_edge_to_edge_in_one_hairline_box() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let at = |tab: Tab| hits.rect_of(&Target::Tab(tab)).expect("a tab");
    let (overview, diff, files) = (at(Tab::Overview), at(Tab::Diff), at(Tab::Files));
    assert!(
        (overview.right() - diff.x).abs() < 0.5,
        "no gap between them"
    );
    assert!((diff.right() - files.x).abs() < 0.5);
    let styles = Styles::new(app.config.theme(), Tokens::new(1.0));
    let quads = frame.layers().iter().flat_map(|one| one.quads.iter());
    let boxed = quads.clone().any(|quad| {
        quad.shape.stroke > 0.0
            && quad.color == styles.line()
            && (quad.rect.x - overview.x).abs() < 0.5
            && (quad.rect.right() - files.right()).abs() < 0.5
    });
    assert!(boxed, "one box around the set");
    let held = quads
        .into_iter()
        .find(|quad| quad.rect == diff && quad.color == styles.glass_chosen());
    assert_eq!(held.expect("the open one held").shape.radius, 0.0, "square");
    let texts = crate::tests::all_texts(&frame);
    assert!(
        texts.iter().any(|one| one == "DIFF"),
        "a word in capitals: {texts:?}"
    );
}
