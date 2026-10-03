//! A note over several lines reads back over all of them, on either forge.

use wiremock::matchers::method;
use wiremock::{Mock, ResponseTemplate};

use super::{github, gitlab};

#[tokio::test]
async fn a_github_thread_over_lines_reads_from_its_first_to_its_last() {
    let mut pr = github::pr();
    pr["reviewThreads"]["nodes"][0]["startLine"] = 10.into();
    let (_server, client) = github::github(github::by_branch(vec![pr])).await;
    let read = client.find_mr(&github::repo(), "b").await.unwrap().unwrap();
    let at = read.threads[0].notes[0]
        .position
        .clone()
        .expect("a position");
    assert_eq!((at.new_line, at.end_new_line), (Some(10), Some(12)));
}

#[tokio::test]
async fn a_gitlab_discussion_over_lines_reads_its_range_from_rest() {
    let (server, client) = gitlab::gitlab(gitlab::by_branch(vec![gitlab::mr("opened")])).await;
    let discussions = serde_json::json!([{
        "id": "abc",
        "notes": [{ "position": { "line_range": {
            "start": { "new_line": 12 }, "end": { "new_line": 15 }
        }}}]
    }]);
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(discussions))
        .mount(&server)
        .await;
    let read = client.find_mr(&gitlab::repo(), "b").await.unwrap().unwrap();
    let at = read.threads[0].notes[0]
        .position
        .clone()
        .expect("a position");
    assert_eq!((at.new_line, at.end_new_line), (Some(12), Some(15)));
}

#[tokio::test]
async fn a_gitlab_read_stands_when_its_discussions_do_not_answer() {
    let (_server, client) = gitlab::gitlab(gitlab::by_branch(vec![gitlab::mr("opened")])).await;
    let read = client.find_mr(&gitlab::repo(), "b").await.unwrap().unwrap();
    let at = read.threads[0].notes[0]
        .position
        .clone()
        .expect("a position");
    assert_eq!((at.new_line, at.end_new_line), (Some(12), None));
}
