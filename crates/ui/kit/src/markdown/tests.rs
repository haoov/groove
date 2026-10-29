use super::parse::{Bullet, Kind, blocks};
use super::plain;

fn kinds(text: &str) -> Vec<(Kind, usize)> {
    blocks(text)
        .iter()
        .map(|one| (one.kind, one.depth))
        .collect()
}

#[test]
fn lists_nest_and_number_from_where_they_start() {
    let found = kinds("3. three\n4. four\n   - under\n");
    assert_eq!(
        found,
        [
            (Kind::Item(Bullet::Number(3)), 1),
            (Kind::Item(Bullet::Number(4)), 1),
            (Kind::Item(Bullet::Dot), 2),
        ]
    );
}

#[test]
fn a_code_block_holds_a_block_per_line_and_no_emphasis() {
    let found = blocks("```\n**a**\nb\n```\n");
    let texts: Vec<&str> = found[0].spans.iter().map(|one| one.text.as_str()).collect();
    assert_eq!(texts, ["**a**", "b"]);
    assert!(found[0].spans.iter().all(|one| !one.strong));
}

#[test]
fn a_quote_marks_its_blocks_quoted() {
    let found = blocks("> said\n\nafter\n");
    assert!(found[0].quoted);
    assert!(!found[1].quoted);
}

#[test]
fn the_plain_first_block_drops_its_marks() {
    assert_eq!(
        plain("**issue:** the `drop` [here](x)\n\nmore"),
        "issue: the drop here"
    );
}

#[test]
fn a_single_line_break_stays_a_break_as_on_the_forges() {
    let found = blocks("first line\nsecond line");
    let texts: Vec<&str> = found[0].spans.iter().map(|one| one.text.as_str()).collect();
    assert_eq!(texts, ["first line", "\n", "second line"]);
}

#[test]
fn a_table_keeps_its_rows_and_each_span_its_cell() {
    let found = blocks("| a | b |\n|---|---|\n| one | **two** |\n");
    assert_eq!(found[0].kind, Kind::Row { head: true });
    assert_eq!(found[1].kind, Kind::Row { head: false });
    let cells: Vec<(usize, &str)> = found[1]
        .spans
        .iter()
        .map(|one| (one.cell, one.text.as_str()))
        .collect();
    assert_eq!(cells, [(0, "one"), (1, "two")]);
    assert_eq!(plain("| a | b |\n|---|---|\n"), "a  ·  b");
}
