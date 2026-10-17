use std::path::{Path, PathBuf};

use serde_json::json;

use crate::position::Encoding;
use crate::{frame, protocol, uri};

#[tokio::test]
async fn a_frame_reads_back_what_was_written() {
    let mut wire = Vec::new();
    let message = json!({ "jsonrpc": "2.0", "method": "é", "params": [1] });
    frame::write(&mut wire, &message).await.unwrap();
    let mut reader = tokio::io::BufReader::new(wire.as_slice());
    assert_eq!(frame::read(&mut reader).await.unwrap(), Some(message));
    assert_eq!(frame::read(&mut reader).await.unwrap(), None);
}

#[tokio::test]
async fn a_frame_without_a_length_is_refused() {
    let mut reader = tokio::io::BufReader::new(&b"Content-Type: x\r\n\r\n{}"[..]);
    assert!(frame::read(&mut reader).await.is_err());
}

#[test]
fn a_path_survives_its_uri() {
    let path = Path::new("/work/my tree/café.py");
    let uri = uri::of(path);
    assert_eq!(uri, "file:///work/my%20tree/caf%C3%A9.py");
    assert_eq!(uri::path(&uri), Some(path.to_path_buf()));
}

#[test]
fn only_a_file_uri_names_a_path() {
    assert_eq!(uri::path("vendored://stdlib/datetime.pyi"), None);
    assert_eq!(uri::path("file:///a"), Some(PathBuf::from("/a")));
}

#[test]
fn each_encoding_counts_a_column_in_its_own_units() {
    let line = "é😀x";
    assert_eq!(Encoding::Utf8.units(line, 2), 6);
    assert_eq!(Encoding::Utf16.units(line, 2), 3);
    assert_eq!(Encoding::Utf32.units(line, 2), 2);
    assert_eq!(Encoding::Utf16.column(line, 3), 2);
    assert_eq!(Encoding::Utf8.column(line, 6), 2);
}

#[test]
fn units_past_the_line_end_count_as_characters() {
    assert_eq!(Encoding::Utf16.column("ab", 5), 5);
}

#[test]
fn a_server_naming_no_encoding_counts_utf16() {
    assert_eq!(Encoding::named(None), Encoding::Utf16);
    assert_eq!(Encoding::named(Some("utf-32")), Encoding::Utf32);
}

#[test]
fn a_dotted_section_reads_its_settings() {
    let settings = json!({ "yaml": { "schemas": { "kubernetes": "*.yaml" } } });
    let schemas = protocol::section(&settings, Some("yaml.schemas"));
    assert_eq!(schemas, json!({ "kubernetes": "*.yaml" }));
    assert_eq!(protocol::section(&settings, Some("ty")), json!(null));
    assert_eq!(protocol::section(&settings, None), settings);
}
