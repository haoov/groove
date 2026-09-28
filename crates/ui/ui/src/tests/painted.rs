//! What the colour cache keeps, and when it gives it up.

use groove_controllers::workspace_service::Document;

use crate::views::session::diff::painted::Painted;

fn doc() -> Document {
    let text: String = (0..200)
        .map(|at| format!("fn name_{at}() -> usize {{ {at} }}\n"))
        .collect();
    Document::new("src/lib.rs", &text)
}

#[test]
fn the_same_rows_are_read_once() {
    let (painted, doc) = (Painted::default(), doc());
    let first = painted.of(&doc, "src/lib.rs", false, (1, (0, 0)), 10..50);
    let again = painted.of(&doc, "src/lib.rs", false, (1, (0, 0)), 10..50);
    assert!(std::rc::Rc::ptr_eq(&first, &again), "read again");
    assert!(!first.of(20).is_empty(), "the line is coloured");
}

#[test]
fn a_small_scroll_keeps_what_it_has_and_a_long_one_does_not() {
    let (painted, doc) = (Painted::default(), doc());
    let first = painted.of(&doc, "src/lib.rs", false, (1, (0, 0)), 60..100);
    let near = painted.of(&doc, "src/lib.rs", false, (1, (0, 0)), 70..110);
    assert!(
        std::rc::Rc::ptr_eq(&first, &near),
        "still inside what it read"
    );
    let far = painted.of(&doc, "src/lib.rs", false, (1, (0, 0)), 160..200);
    assert!(!std::rc::Rc::ptr_eq(&first, &far));
}

#[test]
fn a_document_that_moved_is_read_again() {
    let (painted, doc) = (Painted::default(), doc());
    let first = painted.of(&doc, "src/lib.rs", false, (1, (0, 0)), 10..50);
    let typed = painted.of(&doc, "src/lib.rs", false, (1, (1, 0)), 10..50);
    assert!(!std::rc::Rc::ptr_eq(&first, &typed), "the buffer moved on");
    let loaded = painted.of(&doc, "src/lib.rs", false, (2, (1, 0)), 10..50);
    assert!(
        !std::rc::Rc::ptr_eq(&typed, &loaded),
        "the file was read again"
    );
}

/// The colour of the first run that reads `word`, in a frame of `ui`, its cache kept.
fn colour_of(
    app: &groove_controllers::AppState,
    ui: &crate::Ui,
    word: &str,
) -> Option<groove_gfx::Color> {
    let fonts = &mut groove_gfx::Fonts::embedded();
    let (frame, _) = crate::view(app, ui, crate::tests::window(), fonts);
    let texts = &frame.layers()[0].texts;
    texts.iter().find(|t| t.text == word).map(|t| t.style.color)
}

#[test]
fn a_word_typed_takes_its_colour_once_the_parse_lands() {
    let mut ui = crate::Ui::default();
    ui.session.tab = crate::views::session::Tab::Files;
    let typed = "fn one() {}\n";
    let mut fresh = crate::tests::full_app();
    crate::tests::shows(&mut fresh, "src/lib.rs", "x\n", &format!("{typed}x\n"));
    let wanted = colour_of(&fresh, &ui.clone(), "fn");
    assert!(wanted.is_some(), "the keyword is its own run");

    let mut app = crate::tests::full_app();
    crate::tests::shows(&mut app, "src/lib.rs", "x\n", "x\n");
    let open = app.workspace.active_mut().expect("the open file");
    open.new.edit(&groove_types::Edit::Insert(typed.into()));
    assert_ne!(colour_of(&app, &ui, "fn"), wanted, "not parsed yet");

    let open = app.workspace.active().expect("the open file");
    let revision = open.new.revision();
    let read = groove_controllers::workspace_service::derived(
        "src/lib.rs",
        &open.old,
        open.new.document().clone(),
    );
    let worktree = app.workspace.worktree.clone().expect("a worktree");
    app.workspace
        .derived((&worktree, "src/lib.rs"), read, revision);
    assert_eq!(
        colour_of(&app, &ui, "fn"),
        wanted,
        "coloured with no key after it"
    );
}
