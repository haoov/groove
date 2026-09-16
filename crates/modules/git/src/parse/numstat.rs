/// One row of `git diff --numstat -z`: the lines added and deleted, and the path they
/// belong to. A binary file counts nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counts {
    pub added: Option<u32>,
    pub deleted: Option<u32>,
    pub path: String,
}

impl Counts {
    pub fn is_binary(&self) -> bool {
        self.added.is_none() && self.deleted.is_none()
    }
}

/// `-z` ends every row with a NUL and quotes no path. A row reads
/// `added<TAB>deleted<TAB>path`, and a file that moved leaves the path empty and
/// writes its old and new names as the two rows after it.
pub fn numstat(text: &str) -> Vec<Counts> {
    let mut rows = Vec::new();
    let mut fields = text.split('\0').filter(|field| !field.is_empty());
    while let Some(field) = fields.next() {
        let Some((added, deleted, path)) = row(field) else {
            continue;
        };
        let path = match path.is_empty() {
            true => moved(&mut fields),
            false => path.to_string(),
        };
        if path.is_empty() {
            continue;
        }
        rows.push(Counts {
            added,
            deleted,
            path,
        });
    }
    rows
}

/// The new name of a file that moved: the second of the two names that follow.
fn moved<'a>(fields: &mut impl Iterator<Item = &'a str>) -> String {
    fields.next();
    fields.next().unwrap_or_default().to_string()
}

/// `12<TAB>3<TAB>path`, the counts `-` for a binary file.
fn row(field: &str) -> Option<(Option<u32>, Option<u32>, &str)> {
    let mut parts = field.splitn(3, '\t');
    let added = count(parts.next()?)?;
    let deleted = count(parts.next()?)?;
    Some((added, deleted, parts.next().unwrap_or_default()))
}

fn count(text: &str) -> Option<Option<u32>> {
    match text {
        "-" => Some(None),
        _ => text.parse().ok().map(Some),
    }
}
