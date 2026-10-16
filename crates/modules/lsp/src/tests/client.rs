use std::path::{Path, PathBuf};
use std::time::Duration;

use groove_types::{Caret, ErrorKind, Severity};
use serde_json::{Value, json};

use super::fake::{ROOT, connected, pair};
use crate::{Encoding, Hooks, Launch, start};

#[tokio::test]
async fn the_handshake_offers_utf32_and_names_the_root() {
    let settings = json!({ "ty": { "diagnosticMode": "openFilesOnly" } });
    let (connecting, mut fake, _heard) = pair(settings.clone());
    let initialize = fake.expect("initialize").await;
    let params = &initialize["params"];
    assert_eq!(params["rootUri"], "file:///work/tree");
    assert_eq!(params["workspaceFolders"][0]["name"], "tree");
    let offered = &params["capabilities"]["general"]["positionEncodings"];
    assert_eq!(offered, &json!(["utf-32", "utf-16"]));
    let chosen = json!({ "capabilities": { "positionEncoding": "utf-32" } });
    fake.answer(&initialize, chosen).await;
    let client = connecting.await.unwrap().unwrap();
    assert_eq!(client.encoding(), Encoding::Utf32);
    fake.expect("initialized").await;
    let configured = fake.expect("workspace/didChangeConfiguration").await;
    assert_eq!(configured["params"]["settings"], settings);
}

#[tokio::test]
async fn a_server_naming_no_encoding_gets_its_columns_in_utf16() {
    let (client, mut fake, _heard) = connected(None, Value::Null).await;
    let path = Path::new("/work/tree/a.py");
    client
        .open(path, "python", "é😀x = 1\n".into())
        .await
        .unwrap();
    let opened = fake.expect("textDocument/didOpen").await;
    assert_eq!(opened["params"]["textDocument"]["languageId"], "python");
    let asking = tokio::spawn(async move { client.hover(path, Caret::new(0, 2)).await });
    let request = fake.expect("textDocument/hover").await;
    assert_eq!(
        request["params"]["position"],
        json!({ "line": 0, "character": 3 })
    );
    fake.answer(&request, Value::Null).await;
    assert_eq!(asking.await.unwrap().unwrap(), None);
}

#[tokio::test]
async fn each_change_sends_the_whole_text_at_a_higher_version() {
    let (client, mut fake, _heard) = connected(Some("utf-32"), Value::Null).await;
    let path = Path::new("/work/tree/a.py");
    client.open(path, "python", "a\n".into()).await.unwrap();
    fake.expect("textDocument/didOpen").await;
    for (version, text) in [(2, "ab\n"), (3, "abc\n")] {
        client.change(path, text.into()).await.unwrap();
        let changed = fake.expect("textDocument/didChange").await;
        assert_eq!(changed["params"]["textDocument"]["version"], version);
        assert_eq!(
            changed["params"]["contentChanges"],
            json!([{ "text": text }])
        );
    }
}

#[tokio::test]
async fn published_diagnostics_reach_the_hook_in_characters() {
    let (client, mut fake, mut heard) = connected(None, Value::Null).await;
    let path = PathBuf::from("/work/tree/a.py");
    client
        .open(&path, "python", "é😀x = None\n".into())
        .await
        .unwrap();
    fake.expect("textDocument/didOpen").await;
    let range =
        json!({ "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 4 } });
    let found =
        json!([{ "range": range, "severity": 2, "message": "m", "source": "ty", "code": 101 }]);
    let params = json!({ "uri": "file:///work/tree/a.py", "diagnostics": found });
    fake.send(
        json!({ "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": params }),
    )
    .await;
    let (named, found) = heard.diagnostics.recv().await.unwrap();
    assert_eq!(named, path);
    assert_eq!(
        (found[0].start, found[0].end),
        (Caret::new(0, 2), Caret::new(0, 3))
    );
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(found[0].code.as_deref(), Some("101"));
    assert_eq!(found[0].source.as_deref(), Some("ty"));
}

#[tokio::test]
async fn asking_about_a_file_not_open_is_refused() {
    let (client, _fake, _heard) = connected(None, Value::Null).await;
    let err = client
        .hover(Path::new("/work/tree/b.py"), Caret::new(0, 0))
        .await;
    assert_eq!(err.unwrap_err().kind, ErrorKind::Invalid);
}

#[tokio::test]
async fn a_closed_server_fails_what_waits_on_it_and_says_so() {
    let (client, mut fake, heard) = connected(None, Value::Null).await;
    let path = Path::new("/work/tree/a.py");
    client.open(path, "python", "x\n".into()).await.unwrap();
    fake.expect("textDocument/didOpen").await;
    let asking = tokio::spawn(async move { client.definition(path, Caret::new(0, 0)).await });
    fake.expect("textDocument/definition").await;
    drop(fake);
    let err = asking.await.unwrap().unwrap_err();
    assert_eq!(err.kind, ErrorKind::Io);
    tokio::time::timeout(Duration::from_secs(5), heard.closed)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn start_reports_a_server_that_is_not_installed() {
    let launch = Launch {
        program: "groove-no-such-server".into(),
        args: vec![],
        unset: vec!["VIRTUAL_ENV".into()],
        root: PathBuf::from(ROOT),
        settings: Value::Null,
    };
    let hooks = Hooks {
        on_diagnostics: Box::new(|_, _| {}),
        on_closed: Box::new(|| {}),
    };
    let Err(err) = start(launch, hooks).await else {
        panic!("a missing server started");
    };
    assert_eq!(err.kind, ErrorKind::Io);
}
