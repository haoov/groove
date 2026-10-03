//! What a click tells the program, in either encoding it asked for.

#[test]
fn the_older_encoding_sends_one_raw_byte_a_coordinate_far_into_the_screen() {
    let sent = crate::report(crate::LEFT, (149, 39), true, false);
    assert_eq!(sent, [0x1b, b'[', b'M', 32, 32 + 150, 32 + 40]);
    let far = crate::report(crate::LEFT, (400, 0), true, false);
    assert_eq!(
        far[4], 255,
        "a coordinate past 223 stays at the last one it can name"
    );
}

#[test]
fn the_sgr_encoding_spells_the_cell_out() {
    let sent = crate::report(crate::LEFT, (149, 39), false, true);
    assert_eq!(sent, b"\x1b[<0;150;40m");
}
