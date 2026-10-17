use std::path::Path;

use groove_types::Caret;
use serde_json::{Value, json};

use super::fake::{Fake, pair};
use crate::Client;
use crate::completion::plain;

const PATH: &str = "/work/tree/a.py";

/// A client on a server that counts in UTF-16 and takes changes as ranges.
async fn incremental(text: &str) -> (Client, Fake) {
    let (connecting, mut fake, _heard) = pair(Value::Null);
    let initialize = fake.expect("initialize").await;
    let completion = &initialize["params"]["capabilities"]["textDocument"]["completion"];
    assert_eq!(completion["completionItem"]["snippetSupport"], false);
    let sync = json!({ "capabilities": { "textDocumentSync": { "change": 2 } } });
    fake.answer(&initialize, sync).await;
    let client = connecting.await.unwrap().unwrap();
    fake.expect("initialized").await;
    fake.expect("workspace/didChangeConfiguration").await;
    client.open(Path::new(PATH), "python", text.into()).unwrap();
    fake.expect("textDocument/didOpen").await;
    (client, fake)
}

fn range(from: (u64, u64), to: (u64, u64)) -> Value {
    json!({ "start": { "line": from.0, "character": from.1 }, "end": { "line": to.0, "character": to.1 } })
}

#[tokio::test]
async fn a_change_sends_only_the_range_it_replaced_in_the_server_s_units() {
    let (client, mut fake) = incremental("a = '😀'\nb = 1\n").await;
    let path = Path::new(PATH);
    client.change(path, "a = '😀x'\nb = 1\n".into()).unwrap();
    let typed = fake.expect("textDocument/didChange").await;
    assert_eq!(typed["params"]["textDocument"]["version"], 2);
    assert_eq!(
        typed["params"]["contentChanges"],
        json!([{ "range": range((0, 7), (0, 7)), "text": "x" }])
    );
    client.change(path, "b = 1\n".into()).unwrap();
    let cut = fake.expect("textDocument/didChange").await;
    assert_eq!(
        cut["params"]["contentChanges"],
        json!([{ "range": range((0, 0), (1, 0)), "text": "" }])
    );
}

#[tokio::test]
async fn a_completion_reads_a_list_its_edits_and_its_snippets() {
    let (client, mut fake) = incremental("os.pa\n").await;
    let asking =
        tokio::spawn(async move { client.completion(Path::new(PATH), Caret::new(0, 5)).await });
    let request = fake.expect("textDocument/completion").await;
    assert_eq!(
        request["params"]["position"],
        json!({ "line": 0, "character": 5 })
    );
    let items = json!([
        { "label": "path", "detail": "<module 'os.path'>", "sortText": " 1" },
        { "label": "pathconf", "insertTextFormat": 2,
          "textEdit": { "range": range((0, 3), (0, 5)), "newText": "pathconf(${1:path}, $2)$0" } },
    ]);
    fake.answer(&request, json!({ "isIncomplete": true, "items": items }))
        .await;
    let found = asking.await.unwrap().unwrap();
    assert_eq!(
        (
            found[0].insert.as_str(),
            found[0].from,
            found[0].sort.as_str()
        ),
        ("path", None, " 1")
    );
    assert_eq!(found[0].detail.as_deref(), Some("<module 'os.path'>"));
    assert_eq!(found[1].insert, "pathconf(path, )");
    assert_eq!(found[1].from, Some(Caret::new(0, 3)));
}

#[test]
fn a_snippet_types_its_placeholders_and_first_choices() {
    assert_eq!(plain("${1:on}: ${2|push,pull|}\\$x$0"), "on: push$x");
    assert_eq!(plain("f(${1:a${2:b}})"), "f(ab)");
}
