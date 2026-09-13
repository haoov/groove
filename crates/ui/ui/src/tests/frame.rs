use groove_controllers::AppState;
use groove_gfx::{Palette, Size};

use crate::input::{Input, Key, Modifiers, handle};
use crate::{Ui, view};

fn texts(frame: &groove_gfx::Frame) -> Vec<String> {
    frame
        .layers()
        .iter()
        .flat_map(|l| l.texts.iter().map(|t| t.text.clone()))
        .collect()
}

#[test]
fn an_empty_state_draws_the_rail_and_the_hint() {
    let frame = view(
        &AppState::default(),
        &Ui::default(),
        Size::new(1280, 800),
        1.0,
    );
    assert_eq!(frame.layers().len(), 1);
    let rail = &frame.layers()[0].quads[0];
    assert_eq!(rail.rect.w, 220.0);
    assert_eq!(rail.color, Palette::LATTE.mantle);
    let texts = texts(&frame);
    assert!(texts.iter().any(|t| t == "Board"));
    assert!(texts.iter().any(|t| t.starts_with("No session open")));
}

#[test]
fn hidpi_scales_the_layout() {
    let frame = view(
        &AppState::default(),
        &Ui::default(),
        Size::new(2560, 1600),
        2.0,
    );
    assert_eq!(frame.layers()[0].quads[0].rect.w, 440.0);
    assert_eq!(frame.layers()[0].texts[0].style.size, 24.0);
}

#[test]
fn ctrl_k_opens_the_palette_on_its_own_layer_and_escape_closes_it() {
    let app = AppState::default();
    let mut ui = Ui::default();
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    assert!(
        handle(
            Input::Key {
                key: Key::Char('k'),
                mods: ctrl
            },
            &mut ui,
            &app
        )
        .is_none()
    );
    assert!(ui.palette_open);
    let frame = view(&app, &ui, Size::new(1280, 800), 1.0);
    assert_eq!(frame.layers().len(), 2);
    handle(
        Input::Key {
            key: Key::Escape,
            mods: Modifiers::default(),
        },
        &mut ui,
        &app,
    );
    assert!(!ui.palette_open);
}
