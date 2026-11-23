//! An object's YAML edited by hand: a draft started where the session may write, or thrown away.

use groove_types::{Error, FollowKey, SessionId, Target};

use crate::AppState;

/// The object's YAML made writable, a draft started from it as it stands; or read-only again.
pub(crate) fn write(state: &mut AppState, session: &SessionId, key: &FollowKey, on: bool) {
    let yamls = &mut state.cluster.store.follows.yamls;
    if !on {
        return yamls.read_only(key);
    }
    let Some(target) = target(key) else {
        return state.failed(Error::invalid("only one object's YAML is written"));
    };
    let checked = super::writes::allowed(state, session, &target.context).and_then(|allowed| {
        groove_cluster_service::writable(&target, allowed.read_only, &allowed.held)
    });
    if let Err(e) = checked {
        return state.failed(e);
    }
    state.cluster.store.follows.yamls.write(key);
}

/// The one object a key names.
pub(crate) fn target(key: &FollowKey) -> Option<Target> {
    Some(Target {
        context: key.context.clone(),
        kind: key.kind.clone(),
        namespace: key.namespace.clone(),
        name: key.named_one()?.to_string(),
        subresource: None,
    })
}
