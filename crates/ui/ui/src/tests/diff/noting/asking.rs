//! The agent asked about lines of the diff, from the menu a right click opens.

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
