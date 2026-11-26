//! The composer around a reply being typed: its buttons, its lit edge.

use super::*;
use crate::hit::Composing;
use groove_ui_kit::base::style::{Role, Styles};

/// A window wide enough for the composer's buttons.
fn wide() -> crate::Metrics {
    crate::tests::metrics(1920, 1080, 1.0)
}

/// A click on one of the composer's buttons.
fn pressed(app: &AppState, ui: &mut Ui, asked: Composing) -> Vec<groove_controllers::Command> {
    let (_, hits) = view(app, ui, wide(), &mut Fonts::embedded());
    let rect = hits
        .rect_of(&Target::Compose(asked))
        .unwrap_or_else(|| panic!("{asked:?} is drawn"));
    click(rect, ui, app, &hits)
}

#[test]
fn send_sends_the_reply_and_cancel_drops_it() {
    let (app, mut ui) = replying("this leaks");
    let commands = pressed(&app, &mut ui, Composing::Save);
    assert_eq!(
        one_command(commands),
        delivery::Command::Thread(groove_controllers::delivery::ThreadAct::Reply {
            thread: "t1".into(),
            body: "this leaks".into(),
        })
    );
    assert!(ui.session.noting.is_none());
    let (app, mut ui) = replying("this leaks");
    assert!(pressed(&app, &mut ui, Composing::Cancel).is_empty());
    assert!(
        ui.session.noting.is_none(),
        "cancel drops it and asks nothing"
    );
}

#[test]
fn the_field_s_edge_lights_while_it_has_the_keyboard() {
    let (app, mut ui) = replying("this leaks");
    let styles = Styles::new(app.config.theme(), Tokens::new(1.0));
    let lit = |ui: &Ui| {
        let (frame, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        let code = hits.rect_of(&Target::Code).expect("the rows");
        frame
            .layers()
            .iter()
            .flat_map(|layer| layer.quads.iter())
            .filter(|quad| code.contains(quad.rect.x, quad.rect.y))
            .any(|quad| quad.color == styles.color(Role::Accent))
    };
    assert!(lit(&ui), "lit with the keyboard");
    ui.focus = crate::Focus::Agent;
    assert!(!lit(&ui), "quiet without it");
}
