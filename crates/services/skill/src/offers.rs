//! A session's skills menu.

/// One row of a session's skills menu: the skill, what it is sent with, and what the row says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillOffer {
    pub id: String,
    pub args: Option<String>,
    pub label: String,
}

impl crate::State {
    /// A session's skills menu: no `create-task` or `promote-fact`, `convert-explorer` once a source when several.
    pub fn offers(
        &self,
        kind: &groove_types::SessionKind,
        sources: &[groove_types::ProviderId],
    ) -> Vec<SkillOffer> {
        let skills = self.skills_for(kind).into_iter();
        skills.flat_map(|one| offered(one, sources)).collect()
    }
}

fn offered(skill: &groove_types::Skill, sources: &[groove_types::ProviderId]) -> Vec<SkillOffer> {
    let row = |args: Option<String>, label: String| SkillOffer {
        id: skill.id.clone(),
        args,
        label,
    };
    match skill.name.as_str() {
        "create-task" | "promote-fact" => Vec::new(),
        "convert-explorer" if sources.len() > 1 => sources
            .iter()
            .map(|one| {
                row(
                    Some(one.as_str().to_string()),
                    format!("{} in {}", skill.label, one.label()),
                )
            })
            .collect(),
        _ => vec![row(None, skill.label.clone())],
    }
}
