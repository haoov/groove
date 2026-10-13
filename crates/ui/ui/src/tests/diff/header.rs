//! The band above the rows.

use super::*;
use crate::tests::render;

#[test]
fn the_diff_header_counts_the_whole_change_and_names_no_file() {
    let app = opened();
    let ui = on_diff();
    let (frame, hits) = view_of(&app, &ui);
    let band = hits
        .rect_of(&Target::View(DiffView::Inline))
        .expect("the switch");
    let header: Vec<&str> = frame.layers()[0]
        .texts
        .iter()
        .filter(|text| (text.y - band.y).abs() < band.h)
        .map(|text| text.text.as_str())
        .collect();
    assert!(header.contains(&"1 file"), "{header:?}");
    assert!(
        header.contains(&"+1") && header.contains(&"-1"),
        "{header:?}"
    );
    assert!(!header.iter().any(|t| t.contains("lib.rs")), "{header:?}");
}

#[test]
fn the_files_header_names_the_open_file() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "src/lib.rs"), "{drawn:?}");
}

#[test]
fn a_head_names_the_file_first_and_keeps_the_end_of_its_folder() {
    let long = "crates/ui/ui/src/views/session/diff/surface.rs";
    let mut app = with_files();
    crate::tests::shows(&mut app, long, OLD, NEW);
    let drawn = texts(&app, &on_diff());
    assert!(drawn.iter().any(|t| t == "surface.rs"), "{drawn:?}");
    assert!(
        drawn.iter().any(|t| t.ends_with("session/diff")),
        "the folder after it: {drawn:?}"
    );
}

#[test]
fn the_switch_names_the_two_views_and_picks_one() {
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

    let (texts, _) = render(&app, &ui, window());
    for named in ["inline", "split"] {
        assert!(texts.iter().any(|one| one == named), "{named}: {texts:?}");
    }
}

#[test]
fn the_files_tab_has_no_view_to_switch() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    for view in DiffView::ALL {
        assert!(hits.rect_of(&Target::View(view)).is_none(), "{view:?}");
    }
}

#[test]
fn the_header_says_when_the_file_owes_the_disk() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let band = crate::layout::Layout::of(window(), &ui).workspace;
    let marks = |app: &AppState| {
        let (frame, _) = view_of(app, &ui);
        frame.layers()[0]
            .icons
            .iter()
            .filter(|icon| icon.color == styles.color(groove_ui_kit::base::style::Role::Warn))
            .filter(|icon| icon.rect.y < band.y + Tokens::new(1.0).row * 4.0)
            .count()
    };
    assert_eq!(marks(&app), 0, "nothing is owed yet");

    let mut dirty = opened();
    if let Some(open) = dirty.workspace.active_mut() {
        open.new.edit(&groove_types::Edit::Insert("x".into()));
    }
    assert_eq!(marks(&dirty), 1, "and a mark once something is typed");
}
