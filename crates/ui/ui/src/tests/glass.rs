//! Glass: the palette and the hover popup show what lies below them, and nothing else does.

use groove_controllers::AppState;
use groove_gfx::{Fonts, Frame};
use groove_types::Caret;
use groove_ui_kit::base::style::Styles;
use groove_ui_kit::base::tokens::Tokens;

use super::completion::offered;
use super::diagnostics::{PATH, diagnosed};
use super::hover::moved_to;
use crate::hit::Target;
use crate::input::hovering;
use crate::palette::Palette;
use crate::tests::{app, metrics, on_file};
use crate::{Overlay, Ui, view};

fn panes(app: &AppState, ui: &Ui, tick: u64) -> usize {
    let mut at = metrics(1600, 1000, 1.0);
    at.tick = tick;
    let (frame, _): (Frame, _) = view(app, ui, at, &mut Fonts::embedded());
    frame.layers().iter().map(|layer| layer.glass.len()).sum()
}

#[test]
fn the_palette_is_glass() {
    let app = app();
    let ui = Ui {
        overlay: Some(Overlay::Palette(Palette::default())),
        ..Ui::default()
    };
    let rail = panes(&app, &Ui::default(), 0);
    assert_eq!(
        panes(&app, &ui, 0),
        rail + 1,
        "the rail's, then the palette's"
    );
}

#[test]
fn the_hover_popup_is_glass() {
    let at = Caret { line: 0, column: 5 };
    let mut app = diagnosed(groove_types::Severity::Error);
    app.workspace.servers.asked(PATH, at, false);
    app.workspace
        .servers
        .answered(PATH, at, Some("The absence of a value.".into()));
    let mut ui = on_file();
    moved_to(&mut ui, &app, at, metrics(1600, 1000, 1.0));
    let rail = panes(&app, &ui, 0);
    hovering(&mut ui, &app, 0);
    assert_eq!(panes(&app, &ui, 300), rail + 1);
}

#[test]
fn the_completion_list_stays_solid() {
    let app = offered();
    assert_eq!(panes(&app, &on_file(), 0), panes(&app, &Ui::default(), 0));
}

#[test]
fn a_palette_row_under_the_pointer_takes_a_rounded_see_through_ground() {
    let app = app();
    let mut ui = Ui {
        overlay: Some(Overlay::Palette(Palette::default())),
        ..Ui::default()
    };
    let at = metrics(1600, 1000, 1.0);
    let (_, hits) = view(&app, &ui, at, &mut Fonts::embedded());
    let row = hits.rect_of(&Target::PaletteRow(1)).expect("a second row");
    crate::input::hover(&mut ui, &hits, row.x + 1.0, row.y + row.h / 2.0);
    let (frame, _) = view(&app, &ui, at, &mut Fonts::embedded());
    let tokens = Tokens::new(1.0);
    let hover = Styles::new(app.config.theme(), tokens).glass_hover();
    let ground = frame.layers().iter().flat_map(|layer| &layer.quads);
    let ground = ground
        .into_iter()
        .find(|q| q.color == hover)
        .expect("the ground");
    assert_eq!(ground.rect, row);
    assert_eq!(ground.shape.radius, tokens.sm);
}

#[test]
fn the_rail_floats_as_a_pane_of_glass_inside_its_place() {
    let app = app();
    let ui = Ui::default();
    let (frame, _) = view(&app, &ui, metrics(1600, 1000, 1.0), &mut Fonts::embedded());
    let place = crate::layout::Layout::of(metrics(1600, 1000, 1.0), &ui).rail;
    let pane = frame.layers()[0].glass.first().expect("the rail's pane");
    let inset = Tokens::new(1.0).sm;
    assert_eq!(pane.rect, place.pad(groove_gfx::Edges::all(inset)));
    let deep = Styles::new(app.config.theme(), Tokens::new(1.0)).deep();
    let under = &frame.layers()[0].under;
    assert_eq!(
        under.first(),
        Some(&(place, deep)),
        "on the agent pane's ground"
    );
}
