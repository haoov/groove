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
fn the_header_sits_under_the_tabs_and_rules_the_whole_width() {
    let app = opened();
    let ui = on_diff();
    let (frame, _) = view_of(&app, &ui);
    let tokens = Tokens::new(1.0);
    let workspace = crate::layout::Layout::of(window(), &ui).workspace;
    let band = Rect::new(
        workspace.x,
        workspace.y + tokens.row,
        workspace.w,
        tokens.row,
    );
    let styles = crate::style::Styles::new(app.config.theme(), tokens);
    let rule = frame.layers()[0]
        .quads
        .iter()
        .find(|quad| {
            quad.color == styles.line()
                && quad.rect.h == tokens.hairline
                && quad.rect.y == band.bottom() - tokens.hairline
        })
        .expect("a hairline under the header");
    assert_eq!(rule.rect.x, band.x, "from the left edge");
    assert_eq!(rule.rect.w, band.w, "to the right one");
    let path = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text.ends_with("lib.rs"))
        .expect("the path");
    assert_eq!(path.y, band.y, "the band starts where the tabs end");
}

#[test]
fn a_long_path_keeps_its_end() {
    let long = "crates/ui/ui/src/views/session/components/diff.rs";
    let mut app = with_files();
    app.workspace.opened = Some(from_text(long, OLD, NEW));
    let drawn = texts(&app, &on_diff());
    let path = drawn
        .iter()
        .find(|text| text.ends_with("diff.rs"))
        .expect("the name survives: {drawn:?}");
    assert!(path.ends_with("components/diff.rs"), "{path}");
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
}

#[test]
fn the_header_says_when_the_file_owes_the_disk() {
    let app = opened();
    let ui = on_diff();
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let band = crate::layout::Layout::of(window(), &ui).workspace;
    let marks = |app: &AppState| {
        let (frame, _) = view_of(app, &ui);
        frame.layers()[0]
            .icons
            .iter()
            .filter(|icon| icon.color == styles.color(crate::style::Role::Warn))
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
