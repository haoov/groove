//! The band above the rows.

use super::*;

#[test]
fn the_header_names_the_file_and_what_it_changed() {
    let app = opened();
    let drawn = texts(&app, &on_diff());
    assert!(drawn.iter().any(|t| t == "src/lib.rs"), "{drawn:?}");
    assert!(drawn.iter().any(|t| t == "+1"), "what it added: {drawn:?}");
    assert!(drawn.iter().any(|t| t == "-1"), "and what it took away");
}

#[test]
fn a_long_path_keeps_its_end() {
    let long = "crates/ui/ui/src/views/session/diff/surface.rs";
    let mut app = with_files();
    crate::tests::shows(&mut app, long, OLD, NEW);
    let drawn = texts(&app, &on_diff());
    let path = drawn
        .iter()
        .find(|text| text.ends_with("surface.rs"))
        .expect("the name survives: {drawn:?}");
    assert!(path.ends_with("diff/surface.rs"), "{path}");
}

#[test]
fn the_switch_names_the_three_views_and_picks_one() {
    let app = opened();
    let mut ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    for view in DiffView::ALL {
        assert!(
            hits.rect_of(&Target::View(view)).is_some(),
            "{view:?} can be picked"
        );
    }
    let split = hits.rect_of(&Target::View(DiffView::Split)).expect("split");
    assert!(click(split, &mut ui, &app, &hits).is_empty());
    assert_eq!(ui.session.view, DiffView::Split);

    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|one| one.text.clone())
        .collect();
    for named in ["editor", "inline", "split"] {
        assert!(texts.iter().any(|one| one == named), "{named}: {texts:?}");
    }
}

#[test]
fn the_header_says_when_the_file_owes_the_disk() {
    let app = opened();
    let ui = on_diff();
    let styles = crate::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let band = crate::layout::Layout::of(window(), &ui).workspace;
    let marks = |app: &AppState| {
        let (frame, _) = view_of(app, &ui);
        frame.layers()[0]
            .icons
            .iter()
            .filter(|icon| icon.color == styles.color(crate::base::style::Role::Warn))
            .filter(|icon| icon.rect.y < band.y + Tokens::new(1.0).row * 2.0)
            .count()
    };
    assert_eq!(marks(&app), 0, "nothing is owed yet");

    let mut dirty = opened();
    if let Some(open) = dirty.workspace.opened.as_mut() {
        open.new.edit(&groove_types::Edit::Insert("x".into()));
    }
    assert_eq!(marks(&dirty), 1, "and a mark once something is typed");
}
