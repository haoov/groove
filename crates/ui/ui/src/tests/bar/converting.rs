//! Converting a session to a task: one row for each source that is set up.

use super::*;

#[test]
fn a_conversion_files_in_each_source_that_is_set_up() {
    let app = sourced(
        offering(vec![skill("convert-explorer", "convert to task", &[])]),
        true,
        true,
    );
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let word = hits
        .rect_of(&Target::Skills(SessionId::new("a")))
        .expect("the skills word");
    click(word, &mut ui, &app, &hits);

    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let drawn = all_texts(&frame);
    assert!(
        drawn.iter().any(|one| one == "convert to task in GitHub"),
        "{drawn:?}"
    );
    assert!(
        drawn.iter().any(|one| one == "convert to task in Notion"),
        "{drawn:?}"
    );
    assert!(
        !drawn.iter().any(|one| one == "convert to task"),
        "{drawn:?}"
    );

    let row = hits
        .rect_of(&crate::hit::Target::MenuRow(0))
        .expect("a row of it");
    let acted = click(row, &mut ui, &app, &hits);
    assert_eq!(
        acted,
        vec![groove_controllers::Command::Agent(
            groove_controllers::agent::Command::SendSkill {
                session: SessionId::new("a"),
                id: "groove:convert-explorer".into(),
                args: Some("github".into()),
            }
        )]
    );
}

#[test]
fn filing_a_task_and_sharing_a_fact_stay_off_the_menu_and_converting_names_each_source() {
    let app = sourced(
        offering(vec![
            skill("create-task", "create task", &[]),
            skill("promote-fact", "share a fact", &[]),
            skill("convert-explorer", "convert to task", &[]),
        ]),
        true,
        true,
    );
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let word = hits
        .rect_of(&Target::Skills(SessionId::new("a")))
        .expect("the skills word");
    click(word, &mut ui, &app, &hits);

    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let drawn = all_texts(&frame);
    let rows: Vec<&String> = drawn.iter().filter(|one| one.contains(" in ")).collect();
    let mut once = rows.clone();
    once.sort();
    once.dedup();
    assert_eq!(
        rows.len(),
        2,
        "two sources, one skill on the menu: {rows:?}"
    );
    assert!(
        !drawn
            .iter()
            .any(|one| one.starts_with("create task") || one.starts_with("share a fact")),
        "{drawn:?}"
    );
    assert_eq!(
        once.len(),
        rows.len(),
        "no row says what another says: {rows:?}"
    );
}

#[test]
fn one_source_files_without_naming_it() {
    let app = sourced(
        offering(vec![skill("convert-explorer", "convert to task", &[])]),
        false,
        true,
    );
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let word = hits
        .rect_of(&Target::Skills(SessionId::new("a")))
        .expect("the skills word");
    click(word, &mut ui, &app, &hits);
    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let drawn = all_texts(&frame);
    assert!(
        drawn.iter().any(|one| one == "convert to task"),
        "{drawn:?}"
    );
}
