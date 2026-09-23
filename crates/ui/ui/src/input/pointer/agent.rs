//! What a click on the agent's own row does: its skills, or its reload.

use groove_controllers::{AppState, Command, agent};
use groove_types::{ProviderId, SessionId};

use crate::ctx::Metrics;
use crate::hit::{Hits, Target};
use crate::layout::Layout;
use crate::{Corner, Menu, Of, Offer, Ui};

/// The agent's own targets; anything else is not its to answer.
pub(super) fn acted(
    target: &Target,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Option<Vec<Command>> {
    match target {
        Target::Skills(session) => Some(menu(ui, app, hits, session.clone())),
        Target::Reload(session) => Some(reloaded(ui, session.clone(), metrics)),
        _ => None,
    }
}

/// What the agent can be sent, under the word that opened it.
fn menu(ui: &mut Ui, app: &AppState, hits: &Hits, session: SessionId) -> Vec<Command> {
    let Some(open) = app.session.get(&session) else {
        return Vec::new();
    };
    let sources = groove_controllers::task_service::source_ids(app.config.config.as_ref());
    let offered: Vec<Offer> = app
        .agent
        .skills_for(&open.session.kind)
        .iter()
        .flat_map(|one| rows_of(one, &sources))
        .collect();
    if offered.is_empty() {
        return Vec::new();
    }
    let at = hits
        .rect_of(&Target::Skills(session.clone()))
        .map(|word| (word.x, word.y))
        .unwrap_or_default();
    ui.menu = Some(Menu {
        at,
        corner: Corner::BottomLeft,
        of: Of::Skills { session, offered },
    });
    Vec::new()
}

/// One row a skill offers. A task filed in one of several sources gets a row each,
/// naming the source it files in.
fn rows_of(skill: &groove_types::Skill, sources: &[ProviderId]) -> Vec<Offer> {
    if skill.name != "create-task" || sources.len() < 2 {
        return vec![Offer {
            id: skill.id.clone(),
            args: None,
            label: skill.label.clone(),
        }];
    }
    sources
        .iter()
        .map(|one| Offer {
            id: skill.id.clone(),
            args: Some(one.as_str().to_string()),
            label: format!("file in {}", one.label()),
        })
        .collect()
}

/// The agent ended and started again, on the grid its pane holds now.
fn reloaded(ui: &Ui, session: SessionId, metrics: Metrics) -> Vec<Command> {
    let tokens = metrics.tokens();
    let (cols, rows) = Layout::of(metrics, ui).agent_grid(&tokens, metrics.cell);
    vec![Command::Agent(agent::Command::Reload {
        session,
        cols,
        rows,
    })]
}
