use crate::parse::{Change, RemoteUrl, porcelain, unquote_path};

#[test]
fn remote_urls_parse_in_every_form() {
    let want = RemoteUrl {
        host: "gitlab.example.com".into(),
        group: "group/sub".into(),
        project: "proj".into(),
    };
    assert_eq!(
        RemoteUrl::parse("git@gitlab.example.com:group/sub/proj.git").unwrap(),
        want
    );
    assert_eq!(
        RemoteUrl::parse("ssh://git@gitlab.example.com/group/sub/proj.git").unwrap(),
        want
    );
    assert_eq!(
        RemoteUrl::parse("https://oauth2:SECRET@gitlab.example.com/group/sub/proj").unwrap(),
        want
    );
    assert_eq!(
        RemoteUrl::parse("ssh://git@gitlab.example.com:2222/group/sub/proj").unwrap(),
        want
    );
    assert_eq!(want.slug(), "gitlab.example.com/group/sub/proj");
    let gh = RemoteUrl::parse("https://github.com/owner/proj").unwrap();
    assert_eq!((gh.group.as_str(), gh.project.as_str()), ("owner", "proj"));
    assert!(RemoteUrl::parse("/local/path").is_err());
    assert!(RemoteUrl::parse("https://host/proj").is_err(), "no group");
}

fn triples(text: &str) -> Vec<(char, char, String)> {
    porcelain(text)
        .into_iter()
        .map(|c| (c.x, c.y, c.path))
        .collect()
}

#[test]
fn porcelain_rows_read_apart() {
    assert_eq!(
        triples(
            "?? new.txt\nM  staged.txt\n M dirty.txt\nMM both.txt\nA  added.txt\nD  gone.txt\nUU conflict.txt\n"
        ),
        [
            ('?', '?', "new.txt".to_string()),
            ('M', ' ', "staged.txt".to_string()),
            (' ', 'M', "dirty.txt".to_string()),
            ('M', 'M', "both.txt".to_string()),
            ('A', ' ', "added.txt".to_string()),
            ('D', ' ', "gone.txt".to_string()),
            ('U', 'U', "conflict.txt".to_string()),
        ]
    );
    assert_eq!(
        triples("R  old.txt -> new.txt\n"),
        [('R', ' ', "new.txt".to_string())]
    );
    assert_eq!(
        triples("?? \"a b.txt\"\n"),
        [('?', '?', "\"a b.txt\"".to_string())]
    );
    assert_eq!(
        triples(" M src/a b.txt\n"),
        [(' ', 'M', "src/a b.txt".to_string())]
    );
    assert_eq!(triples("??\n"), [('?', '?', String::new())]);
    assert!(porcelain("\n?\n").is_empty());
}

#[test]
fn a_change_knows_its_kind() {
    let c = |x, y| Change {
        x,
        y,
        path: String::new(),
    };
    assert!(c('?', '?').is_untracked() && c('?', '?').is_modified() && !c('?', '?').is_staged());
    assert!(c('M', ' ').is_staged() && !c('M', ' ').is_modified());
    assert!(c(' ', 'M').is_modified() && !c(' ', 'M').is_staged());
    assert!(c('M', 'M').is_staged() && c('M', 'M').is_modified());
}

#[test]
fn quoted_paths_unquote() {
    assert_eq!(
        unquote_path("\"dir/with space/a.rs\""),
        "dir/with space/a.rs"
    );
    assert_eq!(unquote_path("\"a\\tb.rs\""), "a\tb.rs");
    assert_eq!(unquote_path("\"quote\\\"inside.rs\""), "quote\"inside.rs");
    assert_eq!(unquote_path("plain/path.rs"), "plain/path.rs");
    assert_eq!(unquote_path("\"caf\\303\\251.txt\""), "café.txt");
    assert_eq!(unquote_path("\"dir/\\346\\226\\207/a.rs\""), "dir/文/a.rs");
    assert_eq!(unquote_path("\"\\360\\237\\232\\200.md\""), "\u{1f680}.md");
    assert_eq!(
        unquote_path("\"a\\12.txt\""),
        "a12.txt",
        "two digits are not an escape"
    );
}
