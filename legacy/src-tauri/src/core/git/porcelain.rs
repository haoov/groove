//! The one parser for `git status --porcelain` rows, read by the diff summary and the worktree status.

/// One porcelain row: the index letter, the worktree letter, and the path as git
/// wrote it — quoting kept, a rename resolved to its new side.
#[derive(Debug, PartialEq, Eq)]
pub struct Change {
    pub x: char,
    pub y: char,
    pub path: String,
}

/// Parse `git status --porcelain` output. A row too short to carry a path yields an empty `path`.
pub fn parse(text: &str) -> Vec<Change> {
    let mut changes = vec![];
    for line in text.lines() {
        if line.len() < 2 {
            continue;
        }
        let mut letters = line.chars();
        let x = letters.next().unwrap_or(' ');
        let y = letters.next().unwrap_or(' ');
        let path = if line.len() >= 4 {
            let rest = &line[3..];
            // A rename entry reads "orig -> new"; key on the new path.
            rest.rsplit(" -> ").next().unwrap_or(rest).to_string()
        } else {
            String::new()
        };
        changes.push(Change { x, y, path });
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triples(text: &str) -> Vec<(char, char, String)> {
        parse(text)
            .into_iter()
            .map(|c| (c.x, c.y, c.path))
            .collect()
    }

    #[test]
    fn a_rename_keys_on_the_new_path() {
        assert_eq!(
            triples("R  old.txt -> new.txt\n"),
            [('R', ' ', "new.txt".to_string())]
        );
        assert_eq!(
            triples("R  \"old name.txt\" -> \"new name.txt\"\n"),
            [('R', ' ', "\"new name.txt\"".to_string())]
        );
    }

    #[test]
    fn a_quoted_path_keeps_its_quoting() {
        assert_eq!(
            triples("?? \"a b.txt\"\n?? \"uni-\\303\\251.txt\"\n"),
            [
                ('?', '?', "\"a b.txt\"".to_string()),
                ('?', '?', "\"uni-\\303\\251.txt\"".to_string()),
            ]
        );
    }

    #[test]
    fn an_unquoted_path_holding_a_space_survives() {
        assert_eq!(
            triples(" M src/a b.txt\n"),
            [(' ', 'M', "src/a b.txt".to_string())]
        );
    }

    #[test]
    fn untracked_and_the_staged_worktree_combinations_read_apart() {
        assert_eq!(
            triples("?? new.txt\nM  staged.txt\n M dirty.txt\nMM both.txt\nA  added.txt\nD  gone.txt\nUU conflict.txt\n"),
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
    }

    #[test]
    fn a_row_with_no_room_for_a_path_has_none() {
        assert_eq!(triples("??\n"), [('?', '?', String::new())]);
        assert!(parse("\n?\n").is_empty());
    }
}
