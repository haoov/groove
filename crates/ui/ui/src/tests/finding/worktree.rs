//! Searching the worktree: its two terms, and the line found opened where it sits.

use super::*;

#[test]
fn the_chord_opens_the_bar_on_the_worktree_and_typing_searches_it() {
    let app = app();
    let mut ui = on_code();
    press(Key::Char('f'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    assert_eq!(
        ui.session.bar.typing,
        Some(Term::Text),
        "the keyboard is in the text term"
    );

    let commands = press(Key::Char('x'), Modifiers::default(), &mut ui, &app);
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert_eq!(commands[0].id(), "workspace.grep", "as it is typed");
}

#[test]
fn the_two_terms_narrow_together_whichever_is_typed_first() {
    let app = app();
    let mut ui = on_code();
    press(Key::Char('p'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    typed("ui", &mut ui, &app);
    assert_eq!(ui.session.bar.path.text(), "ui");

    press(Key::Char('f'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    assert_eq!(
        ui.session.bar.path.text(),
        "ui",
        "the path is kept as scope"
    );
    let commands = typed("Aligned", &mut ui, &app);
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::Grep {
            query: "Aligned".into(),
            under: "ui".into(),
        })],
        "the text is looked for only under the path"
    );

    press(Key::Char('p'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    assert_eq!(
        ui.session.bar.text.text(),
        "Aligned",
        "and now the other way"
    );
    let commands = typed("src", &mut ui, &app);
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::Grep {
            query: "Aligned".into(),
            under: "src".into(),
        })],
        "a path typed over what was found searches again under it"
    );
}

#[test]
fn a_found_line_opens_its_file_where_it_sits() {
    let mut app = app();
    app.workspace.found = vec![groove_controllers::workspace_service::Found {
        path: "src/b.rs".into(),
        line: 3,
        text: "let one = 22;".into(),
        at: (4, 7),
    }];
    let mut ui = on_code();
    press(Key::Char('f'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    press(Key::Char('o'), Modifiers::default(), &mut ui, &app);

    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&crate::hit::Target::Found(0))
        .expect("the sidebar lists what was found");
    let commands = crate::tests::click(row, &mut ui, &app, &hits);
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::OpenFile {
            path: "src/b.rs".into(),
            at: Some(groove_types::Selection {
                anchor: groove_types::Caret::new(3, 4),
                head: groove_types::Caret::new(3, 7),
            }),
        })],
        "the file, at the line, holding the match"
    );
}

#[test]
fn the_bar_stands_only_once_a_search_asks_for_it() {
    let app = app();
    let mut ui = crate::tests::sidebar_ui();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    for term in [Term::Path, Term::Text] {
        assert!(
            hits.rect_of(&crate::hit::Target::Term(term)).is_none(),
            "{term:?} waits to be asked for"
        );
    }

    press(Key::Char('f'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    assert_eq!(
        ui.session.bar.typing,
        Some(Term::Text),
        "the chord opens it"
    );
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    for term in [Term::Path, Term::Text] {
        let row = hits
            .rect_of(&crate::hit::Target::Term(term))
            .unwrap_or_else(|| panic!("{term:?} stands in the bar"));
        assert!(crate::tests::click(row, &mut ui, &app, &hits).is_empty());
        assert_eq!(ui.session.bar.typing, Some(term), "the click focuses it");
    }
}

#[test]
fn a_click_on_a_term_keeps_what_it_holds() {
    let app = app();
    let mut ui = crate::tests::sidebar_ui();
    press(Key::Char('p'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    typed("src", &mut ui, &app);
    press(Key::Escape, Modifiers::default(), &mut ui, &app);
    press(Key::Char('p'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    typed("ui", &mut ui, &app);
    press(Key::Enter, Modifiers::default(), &mut ui, &app);

    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&crate::hit::Target::Term(Term::Path))
        .expect("the path term");
    crate::tests::click(row, &mut ui, &app, &hits);
    assert_eq!(
        ui.session.bar.path.text(),
        "ui",
        "a click does not spend it"
    );
    assert_eq!(ui.session.bar.typing, Some(Term::Path));
}
