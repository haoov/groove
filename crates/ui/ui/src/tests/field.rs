//! What a one-line field does with a keystroke.

use groove_ui_kit::widgets::Field;

fn typed(text: &str) -> Field {
    let mut field = Field::default();
    for c in text.chars() {
        field.insert(c);
    }
    field
}

#[test]
fn the_caret_moves_through_what_was_typed() {
    let mut field = typed("src/lib");
    assert_eq!(field.before(), "src/lib", "it types at the end");

    field.left();
    field.left();
    field.insert('X');
    assert_eq!(field.text(), "src/lXib", "and puts a character where it is");
    assert_eq!(field.before(), "src/lX");

    field.home();
    field.insert('.');
    assert_eq!(field.text(), ".src/lXib");
    field.end();
    field.insert('!');
    assert_eq!(field.text(), ".src/lXib!");
}

#[test]
fn backspace_takes_what_is_behind_and_delete_what_is_ahead() {
    let mut field = typed("abc");
    field.left();
    field.backspace();
    assert_eq!(field.text(), "ac", "b went, the caret stayed before c");
    field.delete();
    assert_eq!(field.text(), "a");
    field.delete();
    assert_eq!(field.text(), "a", "nothing ahead to take");
    field.backspace();
    assert!(field.is_empty());
    field.backspace();
    assert!(field.is_empty(), "nothing behind either");
}

#[test]
fn a_caret_never_lands_inside_a_character() {
    let mut field = typed("héllo");
    field.home();
    field.right();
    field.insert('-');
    assert_eq!(field.text(), "h-éllo", "counted in characters, not bytes");
    field.end();
    field.backspace();
    assert_eq!(field.text(), "h-éll");
}

#[test]
fn the_text_before_the_caret_moves_with_it() {
    let mut field = typed("staging");
    assert_eq!(field.before(), "staging");
    field.left();
    field.left();
    assert_eq!(field.before(), "stagi");
}

#[test]
fn a_field_draws_its_caret_as_a_bar_and_its_text_alone() {
    use crate::input::{Key, Modifiers};
    use crate::tests::{changed_files, full_app, press, window};
    use groove_ui_kit::base::style::{Role, Styles};
    use groove_ui_kit::base::tokens::Tokens;

    let mut app = full_app();
    changed_files(
        &mut app,
        &[("src/a.rs", "let one = 1;\n", "let one = 2;\n")],
    );
    let mut ui = crate::Ui::default();
    ui.session.tab = crate::views::session::Tab::Diff;
    ui.focus = crate::Focus::Workspace;
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    press(Key::Char('f'), ctrl, &mut ui, &app);
    crate::tests::typed("one", &mut ui, &app);
    let (frame, _) = crate::view(&app, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let texts: Vec<&str> = frame
        .layers()
        .iter()
        .flat_map(|layer| &layer.texts)
        .map(|one| one.text.as_str())
        .collect();
    assert!(texts.contains(&"one"), "{texts:?}");
    assert!(!texts.iter().any(|one| one.contains('\u{2502}')));
    let accent = Styles::new(app.config.theme(), Tokens::new(1.0)).color(Role::Accent);
    let quads = frame.layers().iter().flat_map(|layer| &layer.quads);
    let bars: Vec<_> = quads
        .filter(|quad| quad.color == accent && quad.rect.h > quad.rect.w)
        .collect();
    let mut typed = frame.layers().iter().flat_map(|layer| &layer.texts);
    let one = typed.rfind(|run| run.text == "one").expect("the text");
    let style = one.style;
    let wide = groove_gfx::Fonts::embedded().measure("one", style.font, style.weight, style.size);
    assert!(
        bars.iter()
            .any(|bar| (bar.rect.x - (one.x + wide)).abs() < 0.5),
        "the caret after the text, at {}: {bars:?}",
        one.x + wide
    );
}
