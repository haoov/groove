use std::path::{Path, PathBuf};

use groove_types::Caret;
use serde_json::{Value, json};

use super::fake::{Fake, connected};
use crate::Client;

const PATH: &str = "/work/tree/a.py";

async fn opened(settings: Value) -> (Client, Fake) {
    let (client, mut fake, _heard) = connected(Some("utf-32"), settings).await;
    client
        .open(Path::new(PATH), "python", "x = 1\n".into())
        .await
        .unwrap();
    fake.expect("textDocument/didOpen").await;
    (client, fake)
}

fn range(line: u64, from: u64, to: u64) -> Value {
    json!({ "start": { "line": line, "character": from }, "end": { "line": line, "character": to } })
}

#[tokio::test]
async fn a_definition_reads_locations_and_links_alike() {
    let (client, mut fake) = opened(Value::Null).await;
    let asking =
        tokio::spawn(async move { client.definition(Path::new(PATH), Caret::new(0, 0)).await });
    let request = fake.expect("textDocument/definition").await;
    let answer = json!([
        { "uri": "file:///work/tree/model.py", "range": range(15, 6, 11) },
        { "targetUri": "file:///work/tree/b.py", "targetRange": range(0, 0, 9), "targetSelectionRange": range(2, 4, 8) },
    ]);
    fake.answer(&request, answer).await;
    let found = asking.await.unwrap().unwrap();
    assert_eq!(found[0].path, PathBuf::from("/work/tree/model.py"));
    assert_eq!(
        (found[0].start, found[0].end),
        (Caret::new(15, 6), Caret::new(15, 11))
    );
    assert_eq!(found[1].path, PathBuf::from("/work/tree/b.py"));
    assert_eq!(found[1].start, Caret::new(2, 4));
}

#[tokio::test]
async fn a_definition_outside_any_file_finds_nothing() {
    let (client, mut fake) = opened(Value::Null).await;
    let asking =
        tokio::spawn(async move { client.definition(Path::new(PATH), Caret::new(0, 0)).await });
    let request = fake.expect("textDocument/definition").await;
    let vendored = json!({ "uri": "vendored://stdlib/datetime.pyi", "range": range(0, 0, 1) });
    fake.answer(&request, vendored).await;
    assert!(asking.await.unwrap().unwrap().is_empty());
}

#[tokio::test]
async fn a_hover_joins_its_marked_strings() {
    let (client, mut fake) = opened(Value::Null).await;
    let asking = tokio::spawn(async move { client.hover(Path::new(PATH), Caret::new(0, 0)).await });
    let request = fake.expect("textDocument/hover").await;
    let contents = json!([{ "language": "python", "value": "x: int" }, "An int."]);
    fake.answer(&request, json!({ "contents": contents })).await;
    let said = asking.await.unwrap().unwrap();
    assert_eq!(said.as_deref(), Some("```python\nx: int\n```\n\nAn int."));
}

#[tokio::test]
async fn the_server_reads_its_settings_by_section() {
    let settings = json!({ "yaml": { "schemas": { "kubernetes": "deploy/**" } } });
    let (_client, mut fake) = opened(settings).await;
    let items = json!({ "items": [{ "section": "yaml.schemas" }, { "section": "ty" }] });
    let ask =
        json!({ "jsonrpc": "2.0", "id": 7, "method": "workspace/configuration", "params": items });
    fake.send(ask).await;
    let answer = fake.next().await;
    assert_eq!(answer["id"], 7);
    assert_eq!(
        answer["result"],
        json!([{ "kubernetes": "deploy/**" }, null])
    );
}

#[tokio::test]
async fn a_request_the_client_does_not_handle_is_refused() {
    let (_client, mut fake) = opened(Value::Null).await;
    let ask = json!({ "jsonrpc": "2.0", "id": "x", "method": "workspace/applyEdit", "params": {} });
    fake.send(ask).await;
    let answer = fake.next().await;
    assert_eq!(answer["id"], "x");
    assert_eq!(answer["error"]["code"], -32601);
}

#[tokio::test]
async fn an_error_answer_fails_the_request_with_its_message() {
    let (client, mut fake) = opened(Value::Null).await;
    let asking = tokio::spawn(async move { client.hover(Path::new(PATH), Caret::new(0, 0)).await });
    let request = fake.expect("textDocument/hover").await;
    let error = json!({ "code": -32803, "message": "the file is not indexed yet" });
    fake.send(json!({ "jsonrpc": "2.0", "id": request["id"], "error": error }))
        .await;
    let err = asking.await.unwrap().unwrap_err();
    assert!(err.message.contains("the file is not indexed yet"), "{err}");
}
