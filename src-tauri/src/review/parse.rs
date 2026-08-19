//! Turning git's text into DTOs: the unified-diff parser and its path helpers.
//!
//! Pure functions over strings, split out of `diff.rs` so they can be tested
//! without a repo. The line numbering here is a contract the whole diff UI leans
//! on — see `parse_unified_diff`.

use super::types::{DiffLine, FileDiff, Hunk};

/// Strip git's C-style quoting from a path. Git wraps paths containing special
/// bytes in double quotes with backslash escapes (unless core.quotePath=false);
/// unquote so the path matches the on-disk name and lines up across git outputs.
pub(super) fn unquote_path(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        let inner = &s[1..s.len() - 1];
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some(other) => out.push(other),
                    None => {}
                }
            } else {
                out.push(c);
            }
        }
        out
    } else {
        s.to_string()
    }
}


/// Extract a path from a `--- a/…` / `+++ b/…` diff header body: unquote git's
/// C-style quoting, then strip the `a/`/`b/` prefix. Returns None for /dev/null.
fn parse_diff_path(rest: &str, prefix: &str) -> Option<String> {
    let rest = rest.trim();
    if rest == "/dev/null" {
        return None;
    }
    let unq = unquote_path(rest);
    Some(unq.strip_prefix(prefix).unwrap_or(&unq).to_string())
}


pub(super) fn parse_unified_diff(diff: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = vec![];
    let mut current_file: Option<FileDiff> = None;
    let mut current_hunk: Option<Hunk> = None;
    let mut line_num: i64 = 0;
    let mut a_path: Option<String> = None;

    for raw_line in diff.lines() {
        if raw_line.starts_with("diff --git ") {
            if let Some(mut f) = current_file.take() {
                if let Some(h) = current_hunk.take() {
                    f.hunks.push(h);
                }
                files.push(f);
            }
            current_hunk = None;
            a_path = None;
            // Best-effort fallback path from the header (used for binary/mode-only
            // changes that have no +++/--- lines); refined from those lines below.
            let fallback = raw_line.split(" b/").last().unwrap_or("").to_string();
            current_file = Some(FileDiff {
                path: fallback,
                added: 0,
                deleted: 0,
                status: "M".to_string(),
                staged: None,
                hunks: vec![],
            });
            continue;
        }

        // The `--- a/…` / `+++ b/…` header lines only appear before the first hunk;
        // once inside a hunk a leading "--- "/"+++ " is file content, not a header.
        if current_hunk.is_none() {
            if let Some(rest) = raw_line.strip_prefix("--- ") {
                a_path = parse_diff_path(rest, "a/");
                continue;
            }
            if let Some(rest) = raw_line.strip_prefix("+++ ") {
                let b_path = parse_diff_path(rest, "b/");
                if let Some(f) = current_file.as_mut() {
                    // Deletions show "+++ /dev/null" → fall back to the a-side path.
                    if let Some(p) = b_path.or_else(|| a_path.clone()) {
                        f.path = p;
                    }
                }
                continue;
            }
        }

        if raw_line.starts_with("@@ ") {
            if let Some(f) = current_file.as_mut() {
                if let Some(h) = current_hunk.take() {
                    f.hunks.push(h);
                }
            }
            // Parse new-file start line; start one below so the pre-increment
            // lands on the correct number for the first diff line.
            line_num = parse_hunk_line_num(raw_line).unwrap_or(1) - 1;
            current_hunk = Some(Hunk {
                header: raw_line.to_string(),
                lines: vec![],
            });
            continue;
        }

        let Some(hunk) = current_hunk.as_mut() else {
            continue;
        };

        let (line_type, content) = if let Some(c) = raw_line.strip_prefix('+') {
            line_num += 1;
            ("add", c.to_string())
        } else if let Some(c) = raw_line.strip_prefix('-') {
            ("del", c.to_string())
        } else if let Some(c) = raw_line.strip_prefix(' ') {
            line_num += 1;
            ("ctx", c.to_string())
        } else {
            continue;
        };

        hunk.lines.push(DiffLine {
            num: line_num,
            content,
            line_type: line_type.to_string(),
        });
    }

    if let Some(mut f) = current_file.take() {
        if let Some(h) = current_hunk.take() {
            f.hunks.push(h);
        }
        files.push(f);
    }

    for f in &mut files {
        for h in &f.hunks {
            for l in &h.lines {
                match l.line_type.as_str() {
                    "add" => f.added += 1,
                    "del" => f.deleted += 1,
                    _ => {}
                }
            }
        }
    }

    files
}

fn parse_hunk_line_num(header: &str) -> Option<i64> {
    // "@@ -old,n +new,n @@" → extract new file start line
    let plus_start = header.find('+')? + 1;
    let rest = &header[plus_start..];
    let end = rest.find([',', ' '])?;
    rest[..end].parse().ok()
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Two hunks in one file, an add, a delete, and a replacement.
    const SAMPLE: &str = "\
diff --git a/src/main.rs b/src/main.rs
index 1111111..2222222 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -8,7 +8,8 @@ fn main() {
 ctx eight
 ctx nine
-old ten
+new ten
+added eleven
 ctx twelve
 ctx thirteen
@@ -40,6 +41,5 @@ fn other() {
 ctx forty
-gone
 ctx forty-two
 ctx forty-three
";

    fn nums(h: &Hunk, kind: &str) -> Vec<i64> {
        h.lines.iter().filter(|l| l.line_type == kind).map(|l| l.num).collect()
    }

    #[test]
    fn splits_files_and_hunks() {
        let files = parse_unified_diff(SAMPLE);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/main.rs");
        assert_eq!(files[0].hunks.len(), 2);
    }

    // THE contract: `num` is the NEW-side line number for ctx and add lines. Diff
    // context expansion, annotations and MR threads all anchor on it.
    #[test]
    fn numbers_context_and_added_lines_on_the_new_side() {
        let files = parse_unified_diff(SAMPLE);
        let h = &files[0].hunks[0];
        assert_eq!(nums(h, "ctx"), vec![8, 9, 12, 13]);
        assert_eq!(nums(h, "add"), vec![10, 11]);
    }

    /// A deleted line does not exist on the new side, so it carries the number of
    /// the last new-side line before it — never a number of its own.
    #[test]
    fn a_deleted_line_never_advances_the_new_side_number() {
        let files = parse_unified_diff(SAMPLE);
        let h = &files[0].hunks[0];
        // `-old ten` sits after ctx 9 and before add 10.
        assert_eq!(nums(h, "del"), vec![9]);

        let h2 = &files[0].hunks[1];
        assert_eq!(nums(h2, "ctx"), vec![41, 42, 43]);
        assert_eq!(nums(h2, "del"), vec![41]);
    }

    #[test]
    fn counts_added_and_deleted_lines() {
        let files = parse_unified_diff(SAMPLE);
        assert_eq!(files[0].added, 2);
        assert_eq!(files[0].deleted, 2);
    }

    #[test]
    fn keeps_the_hunk_header_for_the_ui() {
        let files = parse_unified_diff(SAMPLE);
        assert!(files[0].hunks[0].header.starts_with("@@ -8,7 +8,8 @@"));
    }

    #[test]
    fn reads_a_new_file_whose_old_side_is_dev_null() {
        let diff = "\
diff --git a/new.txt b/new.txt
new file mode 100644
--- /dev/null
+++ b/new.txt
@@ -0,0 +1,2 @@
+one
+two
";
        let files = parse_unified_diff(diff);
        assert_eq!(files[0].path, "new.txt");
        assert_eq!(nums(&files[0].hunks[0], "add"), vec![1, 2]);
    }

    /// A deletion has `+++ /dev/null`, so the path must come from the a-side.
    #[test]
    fn reads_a_deleted_file_from_its_old_path() {
        let diff = "\
diff --git a/gone.txt b/gone.txt
deleted file mode 100644
--- a/gone.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-one
-two
";
        let files = parse_unified_diff(diff);
        assert_eq!(files[0].path, "gone.txt");
        assert_eq!(files[0].deleted, 2);
        assert_eq!(files[0].added, 0);
    }

    #[test]
    fn handles_several_files_in_one_diff() {
        let diff = format!("{SAMPLE}\
diff --git a/b.rs b/b.rs
--- a/b.rs
+++ b/b.rs
@@ -1,1 +1,1 @@
-x
+y
");
        let files = parse_unified_diff(&diff);
        assert_eq!(files.len(), 2);
        assert_eq!(files[1].path, "b.rs");
        assert_eq!(files[1].hunks.len(), 1);
    }

    #[test]
    fn ignores_a_binary_file_but_still_lists_it() {
        let diff = "\
diff --git a/logo.png b/logo.png
index 1111111..2222222 100644
Binary files a/logo.png and b/logo.png differ
";
        let files = parse_unified_diff(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "logo.png");
        assert!(files[0].hunks.is_empty(), "no text hunks for a binary file");
    }

    #[test]
    fn empty_input_yields_no_files() {
        assert!(parse_unified_diff("").is_empty());
    }

    #[test]
    fn a_no_newline_marker_is_not_a_diff_line() {
        let diff = "\
diff --git a/a.txt b/a.txt
--- a/a.txt
+++ b/a.txt
@@ -1,1 +1,1 @@
-one
+two
\\ No newline at end of file
";
        let files = parse_unified_diff(diff);
        let h = &files[0].hunks[0];
        assert_eq!(h.lines.len(), 2, "the marker must not become a line");
    }

    #[test]
    fn unquotes_a_path_with_a_space_or_escape() {
        assert_eq!(unquote_path("\"dir/with space/a.rs\""), "dir/with space/a.rs");
        assert_eq!(unquote_path("\"a\\tb.rs\""), "a\tb.rs");
        assert_eq!(unquote_path("\"quote\\\"inside.rs\""), "quote\"inside.rs");
        assert_eq!(unquote_path("plain/path.rs"), "plain/path.rs");
    }

    #[test]
    fn reads_a_quoted_path_from_the_header() {
        let diff = "\
diff --git \"a/dir/with space/a.rs\" \"b/dir/with space/a.rs\"
--- \"a/dir/with space/a.rs\"
+++ \"b/dir/with space/a.rs\"
@@ -1,1 +1,1 @@
-x
+y
";
        let files = parse_unified_diff(diff);
        assert_eq!(files[0].path, "dir/with space/a.rs");
    }
}
