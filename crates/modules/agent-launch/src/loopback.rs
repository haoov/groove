use crate::Result;
use crate::files::LaunchDir;

/// The app's loopback server, as the agent reaches it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loopback {
    pub sse_url: String,
    pub hook_url: String,
    pub token: String,
}

const HOOK_EVENTS: [&str; 6] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "Notification",
    "Stop",
];

impl Loopback {
    /// `--mcp-config` and `--settings`, both through files: they carry the token.
    pub(crate) fn args(&self, files: &LaunchDir) -> Result<Vec<String>> {
        let curl = files.write("hooks.curl", &self.curl_config())?;
        Ok(vec![
            "--mcp-config".into(),
            files.write("mcp.json", &self.mcp_config())?,
            "--settings".into(),
            files.write("settings.json", &self.hook_settings(&curl))?,
        ])
    }

    /// The server name is the agent's tool prefix, `mcp__groove__*`.
    fn mcp_config(&self) -> String {
        serde_json::json!({
            "mcpServers": {
                "groove": {
                    "type": "sse",
                    "url": self.sse_url,
                    "headers": { "Authorization": format!("Bearer {}", self.token) }
                }
            }
        })
        .to_string()
    }

    /// The bearer header as a curl config file, never on a command line.
    fn curl_config(&self) -> String {
        format!("header = \"authorization: Bearer {}\"\n", self.token)
    }

    /// Every hook posts its payload; `-m 2 … || true` keeps a dead server from stalling the agent.
    fn hook_settings(&self, curl_config: &str) -> String {
        let command = format!(
            "curl -s -m 2 -K '{curl_config}' -X POST -H 'content-type: application/json' --data-binary @- '{}' >/dev/null 2>&1 || true",
            self.hook_url
        );
        let post = serde_json::json!([{ "hooks": [{ "type": "command", "command": command }] }]);
        let hooks: serde_json::Map<String, serde_json::Value> = HOOK_EVENTS
            .iter()
            .map(|e| (e.to_string(), post.clone()))
            .collect();
        serde_json::json!({ "hooks": hooks }).to_string()
    }
}
