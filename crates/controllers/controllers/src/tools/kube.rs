//! The agent's kubectl reads: checked against the session's clusters, run in a job, said on the timeline.

use groove_agent_service::Call;
use groove_types::TimelineKind;

use crate::{AppState, Continuation, Services, Spawner};

/// One read of a cluster the session holds, answered from a job.
pub(super) fn read(state: &AppState, services: &Services, spawner: &dyn Spawner, call: Call) {
    let session = super::session(&call);
    let Some(open) = state.session.get(&session) else {
        return call.reply.failed(super::NO_SESSION);
    };
    let Some(context) = call.text("context").map(str::to_string) else {
        return call.reply.failed("kube needs the context to read");
    };
    let Some(args) = call.texts("args") else {
        return call
            .reply
            .failed("kube needs args: the kubectl arguments, a list of strings");
    };
    let checked = match groove_cluster_service::checked(&context, &args, &open.clusters) {
        Ok(checked) => checked,
        Err(e) => return call.reply.failed(e.message),
    };
    let subject = format!("{context} · kubectl {}", args.join(" "));
    crate::timeline::said(
        services,
        spawner,
        &session,
        TimelineKind::ClusterRead,
        subject,
    );
    let filter = call.text("jq").map(str::to_string);
    spawner.spawn(Box::pin(async move {
        match groove_cluster_service::kubectl(&checked, filter.as_deref()).await {
            Ok(answer) if answer.failed => call.reply.failed(answer.text),
            Ok(answer) => call.reply.said(answer.text),
            Err(e) => call.reply.failed(e.message),
        }
        Box::new(|_: &mut AppState, _: &Services, _: &dyn Spawner| {}) as Continuation
    }));
}
