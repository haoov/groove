//! The agent asked about what the diff shows: lines from the menu, or a note from its own button.

use super::*;

#[test]
fn asking_the_agent_about_lines_puts_where_they_stand_in_its_prompt() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    asked(&app, &mut ui, 1);
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&Target::MenuRow(1))
        .expect("the menu's second row");
    assert!(click(row, &mut ui, &app, &hits).is_empty());
    assert_eq!(ui.agent.prompt.text(), "`src/lib.rs:2` ");
    assert_eq!(ui.focus, crate::Focus::Agent);
}

#[test]
fn asking_the_agent_about_a_note_puts_its_place_and_words_in_its_prompt() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let mut ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let origin = own().origin;
    let ask = Target::Note(origin, crate::hit::NoteButton::Ask);
    let button = hits.rect_of(&ask).expect("the note's ask button");
    click(button, &mut ui, &app, &hits);
    let body = own().said[0].body.trim().to_string();
    let place = crate::input::ask::place(own().anchor.as_ref().expect("an anchor"));
    let said = format!("About the note on {place}: \"{body}\"\n");
    assert_eq!(ui.agent.prompt.text(), said);
    assert_eq!(ui.focus, crate::Focus::Agent);
}
