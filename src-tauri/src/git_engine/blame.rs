use sqlx::SqlitePool;

use super::types::BlameLine;

/// Who last touched each line of a file, one entry per line in order.
///
/// Blame describes the file as git sees it on disk, so unsaved edits shift the
/// attribution until the buffer is saved.
#[tauri::command]
pub async fn blame_file(
    worktree_id: String,
    file_path: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<BlameLine>, String> {
    let wt = crate::db::load::worktree(&pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let out = super::run_git_output(&wt.path, &["blame", "--porcelain", "--", &file_path])
        .await
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "git blame {file_path} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(parse_porcelain(&String::from_utf8_lossy(&out.stdout)))
}

/// Metadata porcelain prints once per commit, reused by that commit's later lines.
#[derive(Clone, Default)]
struct CommitMeta {
    author: String,
    time: i64,
    summary: String,
}

/// A header is 40 hex characters then a space. Checking the hex matters: a
/// `committer <long name>` or `filename <path with spaces>` line can also carry a
/// space at offset 40, and would otherwise be read as a new group.
fn is_group_header(raw: &str) -> bool {
    let b = raw.as_bytes();
    b.len() > 40 && b[40] == b' ' && b[..40].iter().all(|c| c.is_ascii_hexdigit())
}

/// Parse `git blame --porcelain`.
///
/// A line group starts with `<sha> <origLine> <finalLine> [numLines]`, then header
/// fields, then the content line prefixed by a tab. Git sends a commit's author and
/// summary only on its FIRST group, so later groups must read them from the map.
fn parse_porcelain(text: &str) -> Vec<BlameLine> {
    let mut meta: std::collections::HashMap<String, CommitMeta> = std::collections::HashMap::new();
    let mut lines: Vec<BlameLine> = vec![];
    let mut sha = String::new();
    let mut line_no: u32 = 0;
    let mut cur = CommitMeta::default();

    for raw in text.lines() {
        // A tab-prefixed line is the file content, and closes the current group.
        if raw.starts_with('\t') {
            if !cur.author.is_empty() {
                meta.insert(sha.clone(), std::mem::take(&mut cur));
            }
            let m = meta.get(&sha).cloned().unwrap_or_default();
            lines.push(BlameLine {
                line: line_no,
                sha: sha.clone(),
                short_sha: sha.chars().take(8).collect(),
                author: m.author,
                time: m.time,
                summary: m.summary,
                uncommitted: sha.chars().all(|c| c == '0'),
            });
            cur = CommitMeta::default();
            continue;
        }

        if let Some(rest) = raw.strip_prefix("author ") {
            cur.author = rest.to_string();
        } else if let Some(rest) = raw.strip_prefix("author-time ") {
            cur.time = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = raw.strip_prefix("summary ") {
            cur.summary = rest.to_string();
        } else if is_group_header(raw) {
            // The group header: sha, original line, final line.
            let mut parts = raw.split(' ');
            sha = parts.next().unwrap_or_default().to_string();
            let _orig = parts.next();
            line_no = parts.next().and_then(|s| s.parse().ok()).unwrap_or(line_no + 1);
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two lines from one commit (metadata printed once), then an uncommitted line.
    const SAMPLE: &str = "\
1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b 1 1 2
author Ada Lovelace
author-mail <ada@example.com>
author-time 1700000000
author-tz +0100
summary feat: add the thing
filename a.rs
\tfirst line
1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b 2 2
\tsecond line
0000000000000000000000000000000000000000 3 3 1
author Not Committed Yet
author-mail <not.committed.yet>
author-time 1700000100
author-tz +0100
summary Version of a.rs from a.rs
filename a.rs
\tthird line
";

    #[test]
    fn parses_line_numbers_and_authors() {
        let lines = parse_porcelain(SAMPLE);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines.iter().map(|l| l.line).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(lines[0].author, "Ada Lovelace");
        assert_eq!(lines[0].summary, "feat: add the thing");
        assert_eq!(lines[0].time, 1700000000);
    }

    #[test]
    fn reuses_metadata_for_a_repeated_commit() {
        let lines = parse_porcelain(SAMPLE);
        assert_eq!(lines[1].author, "Ada Lovelace");
        assert_eq!(lines[1].summary, "feat: add the thing");
        assert_eq!(lines[1].sha, lines[0].sha);
        assert!(!lines[1].uncommitted);
    }

    #[test]
    fn flags_the_all_zero_sha_as_uncommitted() {
        let lines = parse_porcelain(SAMPLE);
        assert!(lines[2].uncommitted);
        assert_eq!(lines[2].short_sha, "00000000");
    }

    #[test]
    fn empty_output_yields_no_lines() {
        assert!(parse_porcelain("").is_empty());
    }

    // A long committer name or a path with a space also puts a space at offset 40.
    #[test]
    fn ignores_fields_that_look_like_a_header() {
        let text = "\
1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b 1 1 1
author Ada Lovelace
author-time 1700000000
summary feat: add the thing
committer GitLab Continuous Integration Service Bot
previous 0b9a8f7e6d5c4b3a2e1d0c9b8a7f6e5d4c3b2a1b dir/a b/c.rs
filename dir/with a space/and another/file.rs
\tonly line
";
        let lines = parse_porcelain(text);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].line, 1);
        assert_eq!(lines[0].author, "Ada Lovelace");
        assert_eq!(lines[0].sha, "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b");
    }
}
