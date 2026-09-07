//! The Notion half of hour logging: read the current number, add, write back.

/// Add `hours` to the page's number property. Re-reads the current value first.
/// Returns `(before, after)`.
pub async fn add_hours(
    token: &str,
    notion_page_id: &str,
    property: &str,
    hours: f64,
) -> anyhow::Result<(f64, f64)> {
    if !(hours.is_finite() && hours > 0.0 && hours < 1000.0) {
        return Err(anyhow::anyhow!(
            "{hours} is not a plausible number of hours"
        ));
    }

    let page = super::api::get(token, &format!("v1/pages/{notion_page_id}")).await?;
    let before = page["properties"][property]["number"]
        .as_f64()
        .unwrap_or(0.0);
    let after = ((before + hours) * 100.0).round() / 100.0;

    super::api::patch(
        token,
        &format!("v1/pages/{notion_page_id}"),
        &serde_json::json!({ "properties": { property: { "number": after } } }),
    )
    .await?;

    Ok((before, after))
}
