use groove_types::RowKind;

use crate::review;

#[test]
fn a_review_lines_up_both_sides_and_colours_them_by_the_path_s_grammar() {
    let before = "spec:\n  replicas: 1\n";
    let after = "spec:\n  replicas: 3\n";
    let one = review("kind/paxone/deployments/api.yaml", before, after);
    let kinds: Vec<RowKind> = one.aligned.rows().map(|row| row.kind).collect();
    assert!(
        kinds.contains(&RowKind::Removed) && kinds.contains(&RowKind::Added),
        "{kinds:?}"
    );
    let colours = one.new.colours(0..2);
    assert!(!colours.of(1).is_empty(), "the YAML grammar paints the key");
}
