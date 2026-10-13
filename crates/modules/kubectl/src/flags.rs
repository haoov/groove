//! The flags a read may carry, and the arguments split into flags and words.

/// Flags that take a value, by every name they go by.
const VALUED: &[&[&str]] = &[
    &["-n", "--namespace"],
    &["-l", "--selector"],
    &["--field-selector"],
    &["-o", "--output"],
    &["--sort-by"],
    &["-L", "--label-columns"],
    &["-c", "--container"],
    &["--since"],
    &["--since-time"],
    &["--tail"],
    &["--limit-bytes"],
    &["--chunk-size"],
    &["--request-timeout"],
    &["--api-group"],
    &["--for"],
    &["--types"],
    &["--revision"],
    &["--subresource"],
    &["--max-log-requests"],
];

/// Flags that take no value, or `=true` and `=false`.
const SWITCHES: &[&[&str]] = &[
    &["-A", "--all-namespaces"],
    &["--show-labels"],
    &["--no-headers"],
    &["-p", "--previous"],
    &["--timestamps"],
    &["--all-containers"],
    &["--prefix"],
    &["--show-kind"],
    &["--ignore-not-found"],
    &["--containers"],
    &["--sum"],
    &["--namespaced"],
    &["--list"],
    &["--show-managed-fields"],
    &["--show-events"],
];

/// One flag as written, by its long name, and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Flag {
    pub name: &'static str,
    pub value: Option<String>,
}

/// The arguments read: the words in order, and the flags.
#[derive(Debug, Default)]
pub(crate) struct Read {
    pub words: Vec<String>,
    pub flags: Vec<Flag>,
}

impl Read {
    pub fn has(&self, name: &str) -> bool {
        self.flags
            .iter()
            .any(|one| one.name == name && one.value.as_deref() != Some("false"))
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        let one = self.flags.iter().rev().find(|one| one.name == name)?;
        one.value.as_deref()
    }
}

/// The arguments split into words and known flags; the first flag no read takes is refused.
pub(crate) fn read(args: &[String]) -> Result<Read, String> {
    let mut read = Read::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == "--" || arg == "-" {
            return Err(format!(
                "{arg} is not taken: a read names no file and no command"
            ));
        }
        if !arg.starts_with('-') {
            read.words.push(arg.clone());
            continue;
        }
        let (written, attached) = split(arg);
        if let Some(name) = long(VALUED, written) {
            let value = match attached {
                Some(value) => value,
                None => rest
                    .next()
                    .cloned()
                    .ok_or(format!("{written} needs a value"))?,
            };
            read.flags.push(Flag {
                name,
                value: Some(value),
            });
        } else if let Some(name) = long(SWITCHES, written) {
            read.flags.push(Flag {
                name,
                value: attached,
            });
        } else {
            return Err(format!("{written} is not a flag a read takes"));
        }
    }
    Ok(read)
}

/// `-ovalue`, `-o=value`, `--name=value`, or the flag alone.
fn split(arg: &str) -> (&str, Option<String>) {
    let short = !arg.starts_with("--") && arg.len() > 2;
    if short && long(VALUED, &arg[..2]).is_some() {
        let value = &arg[2..];
        return (
            &arg[..2],
            Some(value.strip_prefix('=').unwrap_or(value).to_string()),
        );
    }
    match arg.split_once('=') {
        Some((name, value)) => (name, Some(value.to_string())),
        None => (arg, None),
    }
}

/// The long name a flag goes by in `table`.
fn long(table: &[&'static [&'static str]], written: &str) -> Option<&'static str> {
    let names = table.iter().find(|names| names.contains(&written))?;
    names.last().copied()
}
