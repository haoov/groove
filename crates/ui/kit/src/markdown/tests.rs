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
