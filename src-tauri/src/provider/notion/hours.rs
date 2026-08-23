//! The Notion half of hour logging: read the current number, add, write back.
//! The local time ledger (what was measured vs what was sent) lives in
//! `task_manager::hours`, which calls this.

/// Add `hours` to the page's number property. ADDS rather than replaces:
/// "Hours spent" is cumulative, and the value may have been edited in Notion
/// since we last read it, so the current value is re-read immediately before
/// the write. Returns `(before, after)`.
pub async fn add_hours(
    token: &str,
    notion_page_id: &str,
    property: &str,
    hours: f64,
) -> anyhow::Result<(f64, f64)> {
    if !(hours.is_finite() && hours > 0.0 && hours < 1000.0) {
        return Err(anyhow::anyhow!("{hours} is not a plausible number of hours"));
    }

    let page = super::api::get(token, &format!("v1/pages/{notion_page_id}")).await?;
    let before = page["properties"][property]["number"].as_f64().unwrap_or(0.0);
    let after = ((before + hours) * 100.0).round() / 100.0;

    super::api::patch(
        token,
        &format!("v1/pages/{notion_page_id}"),
        &serde_json::json!({ "properties": { property: { "number": after } } }),
    )
    .await?;

    Ok((before, after))
}

/// The number property hours go into. Found in the schema rather than configured:
/// there is exactly one plausible name, and guessing wrong would silently write to
/// the wrong column.
pub async fn hours_property(token: &str, database_id: &str) -> anyhow::Result<String> {
    let schema = super::schema::load(token, database_id).await?;
    schema.hours_property.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "no number property named any of {:?} in this database — nothing to log hours into",
            super::schema::HOURS_NAMES
        )
    })
}
