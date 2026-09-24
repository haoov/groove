use groove_types::TaskKey;

use super::referenced;

const ID: &str = "1f2e3d4c5b6a7980aabbccddeeff0011";

#[test]
fn a_notion_url_names_the_page_it_ends_on() {
    for text in [
        format!("https://www.notion.so/wiremind/Harden-Groove-{ID}"),
        format!("https://www.notion.so/{ID}?pvs=4"),
        "1f2e3d4c-5b6a-7980-aabb-ccddeeff0011".to_string(),
        ID.to_string(),
    ] {
        assert_eq!(
            referenced(&text).expect(&text),
            TaskKey::Notion { page_id: ID.into() },
            "{text}"
        );
    }
}

#[test]
fn an_issue_url_names_the_issue() {
    let key = referenced("https://github.com/haoov/groove/issues/50").expect("an issue");
    assert_eq!(
        key,
        TaskKey::Github {
            host: "github.com".into(),
            owner: "haoov".into(),
            repo: "groove".into(),
            number: 50,
        }
    );
    assert_eq!(
        referenced("github.com/haoov/groove#50").expect("the id Groove writes"),
        key
    );
}

#[test]
fn a_reference_that_names_nothing_is_refused() {
    for text in ["", "a title", "https://github.com/haoov/groove/pull"] {
        assert!(referenced(text).is_err(), "{text}");
    }
}
