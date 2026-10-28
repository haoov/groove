//! The composer around a note being typed: its label chips, its buttons, its lit edge.

use super::*;
use crate::hit::Composing;
use groove_ui_kit::base::style::{Role, Styles};

/// A note open on line 1 of the file view, with `text` typed in it.
fn composing(text: &str) -> (AppState, Ui) {
    let app = opened();
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    ui.session.noting = Some(Noting::new(Anchor::line("src/lib.rs", 1)));
    ui.focus = crate::Focus::Workspace;
    typed(text, &mut ui, &app);
    (app, ui)
}

/// A window wide enough for every chip the composer has.
fn wide() -> crate::Metrics {
    crate::tests::metrics(1920, 1080, 1.0)
}

/// A click on one of the composer's chips or buttons.
fn pressed(app: &AppState, ui: &mut Ui, asked: Composing) -> Vec<groove_controllers::Command> {
    let (_, hits) = view(app, ui, wide(), &mut Fonts::embedded());
    let rect = hits
        .rect_of(&Target::Compose(asked))
        .unwrap_or_else(|| panic!("{asked:?} is drawn"));
    click(rect, ui, app, &hits)
}

fn said(ui: &Ui) -> String {
    ui.session
        .noting
        .as_ref()
        .map(Noting::said)
        .unwrap_or_default()
}

#[test]
fn a_label_chip_writes_the_prefix_and_another_replaces_it() {
    let (app, mut ui) = composing("this leaks");
    pressed(&app, &mut ui, Composing::Label("issue"));
    assert_eq!(said(&ui), "issue: this leaks");
    pressed(&app, &mut ui, Composing::Label("question"));
    assert_eq!(said(&ui), "question: this leaks");
    pressed(&app, &mut ui, Composing::Decoration("blocking"));
    assert_eq!(said(&ui), "question (blocking): this leaks");
    pressed(&app, &mut ui, Composing::Decoration("blocking"));
    assert_eq!(said(&ui), "question: this leaks", "a decoration toggles");
    pressed(&app, &mut ui, Composing::Label("question"));
    assert_eq!(
        said(&ui),
        "this leaks",
        "the chosen label, again, takes it off"
    );
    let (_, hits) = view(&app, &ui, wide(), &mut Fonts::embedded());
    let blocking = Target::Compose(Composing::Decoration("blocking"));
    assert!(
        hits.rect_of(&blocking).is_none(),
        "no decoration without a label"
    );
}

#[test]
fn the_chip_lit_is_the_label_the_words_open_on() {
    let (app, ui) = composing("todo: write the test");
    let styles = Styles::new(app.config.theme(), Tokens::new(1.0));
    let (frame, _) = view(&app, &ui, wide(), &mut Fonts::embedded());
    let colour = |text: &str| {
        frame
            .layers()
            .iter()
            .flat_map(|layer| layer.texts.iter())
            .find(|run| run.text == text)
            .map(|run| run.style.color)
            .expect("the chip")
    };
    assert_eq!(
        colour("todo"),
        styles.color(Role::Warn),
        "typed by hand, it lights"
    );
    assert_eq!(colour("issue"), styles.color(Role::Faint));
}

#[test]
fn save_sends_the_note_and_cancel_drops_it() {
    let (app, mut ui) = composing("this leaks");
    let commands = pressed(&app, &mut ui, Composing::Save);
    assert_eq!(
        one_command(commands),
        delivery::Command::Note(groove_controllers::delivery::NoteAct::Create {
            anchor: Anchor::line("src/lib.rs", 1),
            content: "this leaks".into(),
        })
    );
    assert!(ui.session.noting.is_none());
    let (app, mut ui) = composing("this leaks");
    assert!(pressed(&app, &mut ui, Composing::Cancel).is_empty());
    assert!(
        ui.session.noting.is_none(),
        "cancel drops it and asks nothing"
    );
}

#[test]
fn the_field_s_edge_lights_while_it_has_the_keyboard() {
    let (app, mut ui) = composing("this leaks");
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

#[test]
fn a_narrow_pane_drops_the_chips_it_has_no_room_for_and_never_piles_them() {
    let (app, ui) = composing("this leaks");
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let mut boxes: Vec<groove_gfx::Rect> = groove_types::LABELS
        .iter()
        .filter_map(|one| hits.rect_of(&Target::Compose(Composing::Label(one))))
        .collect();
    boxes.sort_by(|a, b| a.x.total_cmp(&b.x));
    assert!(!boxes.is_empty(), "some chips fit");
    for pair in boxes.windows(2) {
        assert!(pair[1].x >= pair[0].right(), "side by side: {boxes:?}");
    }
}

#[test]
fn a_decoration_chip_keeps_the_agent_s_and_swaps_blocking_for_non_blocking() {
    let (app, mut ui) = composing("issue (agent, blocking): this leaks");
    pressed(&app, &mut ui, Composing::Decoration("non-blocking"));
    assert_eq!(said(&ui), "issue (agent, non-blocking): this leaks");
    pressed(&app, &mut ui, Composing::Label("question"));
    assert_eq!(said(&ui), "question (agent, non-blocking): this leaks");
}
