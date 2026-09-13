/// Replaces the credentials in any `scheme://user:secret@host` with `<redacted>`.
pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("://") {
        let (head, tail) = rest.split_at(at + 3);
        out.push_str(head);
        let (authority, after) = tail.split_at(authority_end(tail));
        out.push_str(&without_credentials(authority));
        rest = after;
    }
    out.push_str(rest);
    out
}

fn authority_end(tail: &str) -> usize {
    tail.find(|c: char| matches!(c, '/' | '\'' | '"' | ')') || c.is_whitespace())
        .unwrap_or(tail.len())
}

fn without_credentials(authority: &str) -> String {
    match authority.rsplit_once('@') {
        Some((_, host)) => format!("<redacted>@{host}"),
        None => authority.to_string(),
    }
}
