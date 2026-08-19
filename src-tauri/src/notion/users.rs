//! Resolving the user by email at setup time.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct NotionUser {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
}

/// Every page is followed: Notion caps a page at 100 entries, bots share the
/// endpoint with people, and the match may sit pages in.
const MAX_USER_PAGES: usize = 30;

/// The workspace person with this email. Requires the integration's
/// "user information with email" capability — without it every email is null
/// and nothing can match.
#[tauri::command]
pub async fn find_notion_user(token: String, email: String) -> Result<NotionUser, String> {
    let wanted = email.trim().to_lowercase();
    if wanted.is_empty() {
        return Err("An email is required.".into());
    }

    let users = super::api::paginate_get(&token, "v1/users", MAX_USER_PAGES)
        .await
        .map_err(|e| e.to_string())?;

    users
        .iter()
        .filter(|u| u["type"].as_str() == Some("person"))
        .find(|u| {
            u["person"]["email"]
                .as_str()
                .is_some_and(|e| e.to_lowercase() == wanted)
        })
        .map(|u| NotionUser {
            id: u["id"].as_str().unwrap_or_default().to_string(),
            name: u["name"].as_str().unwrap_or("(unnamed)").to_string(),
            email: u["person"]["email"].as_str().map(str::to_string),
        })
        .ok_or_else(|| {
            format!(
                "No Notion user with email {email} — check the address, and that the \
                 integration has the \"user information with email\" capability enabled."
            )
        })
}
