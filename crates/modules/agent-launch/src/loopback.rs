//! Where the agent's hooks post, and the token that names the session.

use crate::files::LaunchDir;
use groove_types::Result;

/// The app's loopback server as the agent reaches it; the MCP server is optional.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loopback {
    pub tools: Option<Tools>,
    pub hook_url: String,
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tools {
    pub sse_url: String,
    pub token: String,
}

impl Loopback {
    /// `--mcp-config` and `--settings`, both through files.
    pub(crate) fn args(&self, files: &LaunchDir) -> Result<Vec<String>> {
        let curl = files.write("hooks.curl", &self.curl_config())?;
        let mut args = Vec::new();
        if let Some(tools) = &self.tools {
            args.push("--mcp-config".into());
            args.push(files.write("mcp.json", &tools.mcp_config())?);
        }
        args.push("--settings".into());
        args.push(files.write("settings.json", &self.hook_settings(&curl))?);
        Ok(args)
    }

    /// The bearer header as a curl config file.
    fn curl_config(&self) -> String {
        format!("header = \"authorization: Bearer {}\"\n", self.token)
    }

    /// Every hook posts its payload, giving up after two seconds without failing.
    fn hook_settings(&self, curl_config: &str) -> String {
        let command = format!(
            "curl -s -m 2 -K '{curl_config}' -X POST -H 'content-type: application/json' --data-binary @- '{}' >/dev/null 2>&1 || true",
            self.hook_url
        );
        let post = serde_json::json!([{ "hooks": [{ "type": "command", "command": command }] }]);
        let hooks: serde_json::Map<String, serde_json::Value> = groove_types::HookKind::ALL
            .iter()
            .map(|e| (e.name().to_string(), post.clone()))
            .collect();
        serde_json::json!({ "hooks": hooks }).to_string()
    }
}

impl Tools {
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
}
