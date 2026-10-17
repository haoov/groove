//! The environment check: each program's version, and its sign-in where it has one.

use std::time::Duration;

use groove_exec::run::Run;
use groove_types::{Found, Tool};

const WAIT: Duration = Duration::from_secs(10);

/// How a program says whether it is signed in.
#[derive(Clone, Copy)]
enum Account {
    None,
    /// `auth status` exits zero when signed in.
    Status,
    /// `auth status --json` answers `loggedIn`.
    Json,
}

/// Every program the app runs; `claude` is the path the agent launches.
pub async fn check(claude: &str) -> Vec<Tool> {
    const VERSION: &[&str] = &["--version"];
    let (git, claude, curl, glab, gh, kubectl, jq, ty, yaml) = tokio::join!(
        probe("git", "git", VERSION, Account::None),
        probe("claude", claude, VERSION, Account::Json),
        probe("curl", "curl", VERSION, Account::None),
        probe("glab", "glab", VERSION, Account::Status),
        probe("gh", "gh", VERSION, Account::Status),
        probe(
            "kubectl",
            "kubectl",
            &["version", "--client"],
            Account::None
        ),
        probe("jq", "jq", VERSION, Account::None),
        probe("ty", "ty", VERSION, Account::None),
        probe(
            "yaml-language-server",
            "yaml-language-server",
            VERSION,
            Account::None
        ),
    );
    vec![
        tool("git", "worktrees, diffs, commits", true, git),
        tool("claude", "the agent and its MCP tools", true, claude),
        tool("curl", "the agent's status on the rail", false, curl),
        tool("glab", "GitLab merge requests and CI", false, glab),
        tool("gh", "GitHub pull requests and CI", false, gh),
        tool("kubectl", "the agent's cluster reads", false, kubectl),
        tool("jq", "the agent's filters on a cluster read", false, jq),
        tool("ty", "Python diagnostics, hover and completion", false, ty),
        tool(
            "yaml-language-server",
            "YAML schemas, hover and completion",
            false,
            yaml,
        ),
    ]
}

fn tool(name: &'static str, purpose: &'static str, required: bool, found: Option<Found>) -> Tool {
    Tool {
        name,
        purpose,
        required,
        found,
    }
}

async fn probe(name: &str, program: &str, args: &[&str], account: Account) -> Option<Found> {
    let out = Run::new(program)
        .args(args.iter().copied())
        .timeout(WAIT)
        .text()
        .await;
    let version = version(name, &out.ok()?);
    let signed_in = match account {
        Account::None => None,
        Account::Status => Some(status(program).await),
        Account::Json => Some(json(program).await),
    };
    Some(Found { version, signed_in })
}

async fn status(program: &str) -> bool {
    let run = Run::new(program).args(["auth", "status"]).timeout(WAIT);
    run.output().await.is_ok_and(|out| out.status.success())
}

async fn json(program: &str) -> bool {
    let run = Run::new(program)
        .args(["auth", "status", "--json"])
        .timeout(WAIT);
    run.text().await.is_ok_and(|text| logged_in(&text))
}

/// The first word of the first line that starts with a digit, or that line whole.
pub(crate) fn version(name: &str, text: &str) -> String {
    let line = text.lines().next().unwrap_or_default().trim();
    let words = line.split_whitespace().filter(|word| *word != name);
    let number = words
        .into_iter()
        .find(|w| w.starts_with(|c: char| c.is_ascii_digit()));
    number.unwrap_or(line).to_string()
}

pub(crate) fn logged_in(text: &str) -> bool {
    let value: serde_json::Value = serde_json::from_str(text).unwrap_or_default();
    value["loggedIn"].as_bool().unwrap_or(false)
}
