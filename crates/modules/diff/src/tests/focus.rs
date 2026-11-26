use crate::{At, Changes, HEAD_ROWS, aligned};

fn three() -> Changes {
    Changes::new(vec![
        aligned("src/a.rs", "one\n", "ONE\n"),
        aligned("src/b.rs", "one\ntwo\n", "one\nTWO\nthree\n"),
        aligned("src/c.rs", "x\n", "y\n"),
    ])
}

#[test]
fn a_focused_file_alone_takes_rows() {
    let mut changes = three();
    changes.focus(Some("src/b.rs".into()));
    let b = changes.get("src/b.rs").expect("b").len();
    assert_eq!(changes.rows(), b + HEAD_ROWS);
    assert_eq!(changes.head_of("src/b.rs"), Some(0));
    assert_eq!(
        changes.head_of("src/a.rs"),
        None,
        "an unfocused file has no head"
    );
    assert_eq!(changes.row_of("src/a.rs", 0), None);
    assert!(changes.row_of("src/b.rs", 1).is_some());
    let placed: Vec<&str> = changes
        .placed()
        .map(|(_, _, one)| one.path.as_str())
        .collect();
    assert_eq!(placed, ["src/b.rs"]);
    for row in 0..changes.rows() {
        let path = match changes.at(row) {
            Some(At::Head(one) | At::Row(one, _)) => one.path.as_str(),
            None => "",
        };
        assert_eq!(path, "src/b.rs", "row {row}");
    }
}

#[test]
fn no_focus_shows_every_file_and_an_unknown_one_is_none() {
    let mut changes = three();
    let whole = changes.rows();
    changes.focus(Some("src/c.rs".into()));
    assert!(changes.rows() < whole);
    changes.focus(None);
    assert_eq!(changes.rows(), whole);
    changes.focus(Some("gone.rs".into()));
    assert_eq!(changes.focused(), None);
    assert_eq!(changes.rows(), whole);
}

#[test]
fn the_focus_outlives_a_fold_and_a_read_again() {
    let mut changes = three();
    changes.focus(Some("src/a.rs".into()));
    changes.fold("src/c.rs");
    assert_eq!(changes.focused(), Some("src/a.rs"));
    let mut again = three();
    again.refold(changes.folds(), changes.focused().map(str::to_string));
    assert_eq!(again.focused(), Some("src/a.rs"));
    assert_eq!(
        again.rows(),
        again.get("src/a.rs").expect("a").len() + HEAD_ROWS
    );
}
