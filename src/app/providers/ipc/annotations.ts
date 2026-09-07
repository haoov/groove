import { useStore, findSessionByTask, sessionActions } from '../../../shared/store';
import type { Annotation } from '../../../shared/ipc/ipc';

/** annotation_resolved — resolve it in whichever session holds it. */
export function onAnnotationResolved(payload: { id: string }) {
  const s = useStore.getState();
  for (const id of s.sessionOrder) {
    if (s.sessions[id]?.annotations.some((a) => a.id === payload.id)) {
      sessionActions(id).resolveAnnotation(payload.id);
    }
  }
}

/** annotation_created — the agent left a note; `addAnnotation` dedupes by id. */
export function onAnnotationCreated(payload: Annotation) {
  const sess = findSessionByTask(useStore.getState(), payload.session_id);
  if (sess) sessionActions(sess.id).addAnnotation(payload);
}

/** annotation_updated — the agent rewrote a note's body. */
export function onAnnotationUpdated(payload: Annotation) {
  const sess = findSessionByTask(useStore.getState(), payload.session_id);
  if (sess) sessionActions(sess.id).updateAnnotation(payload.id, payload.content);
}
