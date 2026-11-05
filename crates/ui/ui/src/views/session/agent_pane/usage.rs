//! What the agent uses, in the bar under the prompt: its model, its context, the plan's windows.

use groove_controllers::agent_service::Agent;
use groove_gfx::Rect;
use groove_types::chat::{Chat, usage};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::strip;
use groove_ui_kit::text::Label;

use crate::ctx::Ctx;

/// The line under the prompt, as wide as it.
pub(super) fn draw(ctx: &mut Ctx, chat: Option<&Chat>, agent: &Agent, prompt: Rect) {
    let said = usage(
        chat.and_then(Chat::model),
        chat.and_then(Chat::context),
        agent.limits,
    );
    if said.is_empty() {
        return;
    }
    let bar = ctx.app.layout.agent_bar;
    let line = strip(bar, prompt.x - bar.x, prompt.w);
    Label::new(&said, ctx.styles.small(Role::Faint)).draw(ctx, line);
}
