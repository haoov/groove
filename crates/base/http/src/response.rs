use serde::de::DeserializeOwned;

use crate::{Error, Result, StatusCode};

#[derive(Debug)]
pub struct Response {
    pub method: String,
    pub url: String,
    pub status: StatusCode,
    pub body: String,
}

impl Response {
    /// The response when it succeeded; a failure status becomes an error carrying the body's message.
    pub fn ok(self) -> Result<Self> {
        if self.status == StatusCode::UNAUTHORIZED {
            return Err(Error::Unauthorized {
                method: self.method,
                url: self.url,
            });
        }
        if !self.status.is_success() {
            return Err(Error::Status {
                method: self.method,
                url: self.url,
                status: self.status.as_u16(),
                detail: detail(&self.body),
            });
        }
        Ok(self)
    }

    /// The body as JSON; an empty body is `null`.
    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        let text = if self.body.trim().is_empty() {
            "null"
        } else {
            &self.body
        };
        serde_json::from_str(text).map_err(|source| Error::Decode {
            url: self.url.clone(),
            source,
        })
    }
}

/// The human part of an API error body, whichever key holds it, else the body cut short.
fn detail(body: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    message_of(&value).unwrap_or_else(|| truncate(body.trim(), 300))
}

fn message_of(value: &serde_json::Value) -> Option<String> {
    if let Some(s) = value["message"].as_str() {
        return Some(s.to_string());
    }
    if let Some(list) = value["message"].as_array() {
        let parts: Vec<&str> = list.iter().filter_map(|m| m.as_str()).collect();
        return Some(parts.join("; "));
    }
    value["error"].as_str().map(str::to_string)
}

fn truncate(text: &str, max: usize) -> String {
    let mut short: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        short.push('…');
    }
    short
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
