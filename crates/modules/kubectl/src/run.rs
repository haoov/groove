//! A checked read run: kubectl as an argument list, bounded, its JSON filtered by jq when asked.

use std::time::Duration;

use groove_exec::run::{Capped, Run};
use groove_types::{Error, ErrorKind};

use crate::Checked;

/// How much of an answer the agent reads.
pub const LIMIT: usize = 64 * 1024;

/// How long a read may take.
const TIMEOUT: Duration = Duration::from_secs(30);

/// What a read answered: its text, and whether kubectl or jq refused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub text: String,
    pub failed: bool,
}

/// `checked` run with `kubectl`, then its output through `filter` when one is given.
pub async fn run(kubectl: &str, checked: &Checked, filter: Option<&str>) -> Result<Answer, Error> {
    let ran = Run::new(kubectl)
        .args(checked.argv.iter().cloned())
        .timeout(TIMEOUT);
    let ran = ran.capped(LIMIT).await.map_err(ran_error)?;
    match filter {
        Some(filter) if !ran.cut && ran.success => filtered(filter, ran.stdout).await,
        Some(_) if ran.cut => Ok(cut_before_filter()),
        _ => Ok(answer(ran)),
    }
}

/// The JSON read through jq, with nothing of Groove's environment.
async fn filtered(filter: &str, json: Vec<u8>) -> Result<Answer, Error> {
    if filter.starts_with('-') {
        return Err(Error::invalid("a jq filter starts with no dash"));
    }
    let ran = Run::new("jq")
        .args([filter])
        .bare()
        .input(json)
        .timeout(TIMEOUT);
    Ok(answer(ran.capped(LIMIT).await.map_err(ran_error)?))
}

fn answer(ran: Capped) -> Answer {
    let mut text = String::from_utf8_lossy(&ran.stdout).into_owned();
    if ran.cut {
        text.push_str(&format!(
            "\n… cut at {} KiB: narrow the read with a selector, -o jsonpath, --tail or a jq filter",
            LIMIT / 1024
        ));
    }
    match ran.success || ran.cut {
        true => Answer {
            text,
            failed: false,
        },
        false => Answer {
            text: ran.stderr,
            failed: true,
        },
    }
}

fn cut_before_filter() -> Answer {
    let why = format!(
        "the JSON passed {} KiB before jq could read it whole: narrow the read",
        LIMIT / 1024
    );
    Answer {
        text: why,
        failed: true,
    }
}

fn ran_error(e: groove_exec::Error) -> Error {
    Error::new(ErrorKind::Io, e.to_string())
}
