use crate::{Changes, aligned};

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
    assert_eq!(changes.rows(), b);
    assert_eq!(changes.row_of("src/a.rs", 0), None);
    assert!(changes.row_of("src/b.rs", 1).is_some());
    for row in 0..changes.rows() {
        let path = changes.at(row).map_or("", |(one, _)| one.path.as_str());
        assert_eq!(path, "src/b.rs", "row {row}");
    }
    assert_eq!(changes.at(changes.rows()), None, "past the end");
}

#[test]
fn no_focus_takes_no_rows_and_an_unknown_file_is_none() {
    let mut changes = three();
    assert_eq!(changes.rows(), 0);
    changes.focus(Some("src/c.rs".into()));
    assert!(changes.rows() > 0);
    changes.focus(Some("gone.rs".into()));
    assert_eq!(changes.focused(), None);
    assert_eq!(changes.rows(), 0);
}

#[test]
fn the_focus_outlives_an_edit_of_its_file() {
    let mut changes = three();
    changes.focus(Some("src/a.rs".into()));
    changes.replace(aligned("src/a.rs", "one\n", "TWO\n"));
    assert_eq!(changes.focused(), Some("src/a.rs"));
    assert_eq!(changes.rows(), changes.get("src/a.rs").expect("a").len());
}
