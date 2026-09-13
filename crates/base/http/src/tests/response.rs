use crate::response::*;
use crate::{Error, StatusCode};

fn response(status: u16, body: &str) -> Response {
    Response {
        method: "GET".into(),
        url: "https://h/p".into(),
        status: StatusCode::from_u16(status).unwrap(),
        body: body.into(),
    }
}

#[test]
fn a_failure_carries_the_api_message() {
    let err = response(404, r#"{"message":"Not Found"}"#)
        .ok()
        .unwrap_err();
    assert_eq!(err.to_string(), "GET https://h/p failed 404: Not Found");
}

#[test]
fn gitlab_style_message_lists_are_joined() {
    let err = response(400, r#"{"message":["a","b"]}"#).ok().unwrap_err();
    assert!(err.to_string().ends_with("400: a; b"));
}

#[test]
fn a_body_without_a_message_is_cut_short() {
    let long = "x".repeat(400);
    let err = response(500, &long).ok().unwrap_err();
    assert!(err.to_string().ends_with(&format!("{}…", "x".repeat(300))));
}

#[test]
fn unauthorized_is_its_own_error() {
    assert!(matches!(
        response(401, "").ok(),
        Err(Error::Unauthorized { .. })
    ));
}

#[test]
fn an_empty_body_decodes_as_null() {
    let value: serde_json::Value = response(204, "").json().unwrap();
    assert!(value.is_null());
}
