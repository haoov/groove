import { useState } from 'react';
import { Check, MessageSquare, MessageSquarePlus, Pencil, Send, Trash2 } from 'lucide-react';
import { Markdown } from '../shared/ui/Markdown';
import type { Annotation, Mr, MrThread } from '../shared/ipc/ipc';
import type { AnnCtx, LineRange } from './useAnnotations';

/** Author line for one annotation, colored by the human/agent duet. */
function AnnotationAuthor({ a }: { a: Annotation }) {
  return (
    <span
      className="diff-inline-author"
      style={{ color: a.author === 'agent' ? 'var(--wb-annotation-agent)' : 'var(--wb-annotation-human)' }}
    >
      <MessageSquare size={12} strokeWidth={1.75} className="diff-inline-icon" />
      {a.author}{a.start_line !== a.end_line ? ` · lines ${a.start_line}–${a.end_line}` : ''}
    </span>
  );
}

/**
 * One note with its full control set: post, edit, resolve, delete — and the edit
 * box in place of the body while it is being rewritten. Shared so the note reads
 * and behaves the same wherever it appears.
 */
export function AnnotationRow({ a, ann, mr }: { a: Annotation; ann: AnnCtx; mr: Mr | null }) {
  const editing = ann.editingId === a.id;
  const [confirmDelete, setConfirmDelete] = useState(false);
  return (
    <div className="diff-inline-annotation">
      <AnnotationAuthor a={a} />
      {editing ? (
        <>
          <textarea
            className="diff-annotation-textarea"
            autoFocus
            rows={3}
            value={ann.editText}
            disabled={ann.editPending[a.id]}
            onChange={(e) => ann.setEditText(e.target.value)}
            onKeyDown={(e) => {
              if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') ann.saveEdit();
              if (e.key === 'Escape') { e.stopPropagation(); ann.cancelEdit(); }
            }}
          />
          <div className="diff-inline-annotation-actions">
            <button
              className="diff-inline-post-mr"
              disabled={!ann.editText.trim() || ann.editPending[a.id]}
              onClick={ann.saveEdit}
            >
              {ann.editPending[a.id] ? 'Saving…' : 'Save'}
            </button>
            <button className="diff-inline-delete" onClick={ann.cancelEdit}>Cancel</button>
          </div>
        </>
      ) : (
        <>
          {/* Notes are markdown — the agent writes code spans and lists. */}
          <div className="diff-inline-content">
            <Markdown text={a.content} />
          </div>
          <div className="diff-inline-annotation-actions">
            {confirmDelete ? (
              <>
                <span className="diff-inline-confirm">delete?</span>
                <button
                  className="diff-inline-delete"
                  disabled={ann.deletePending[a.id]}
                  onClick={() => { setConfirmDelete(false); ann.deleteAnnotation(a.id); }}
                >
                  yes
                </button>
                <button className="diff-inline-edit" onClick={() => setConfirmDelete(false)}>no</button>
              </>
            ) : (
              <>
                {mr && a.status === 'open' && (
                  <button
                    className="diff-inline-post-mr"
                    disabled={ann.postPending[a.id]}
                    onClick={() => ann.postToMr(a, mr.id)}
                    title="Publish this note as an MR discussion at its line (resolves the local annotation)"
                  >
                    <Send size={11} strokeWidth={1.75} style={{ marginRight: 4 }} />
                    {ann.postPending[a.id] ? 'Posting…' : 'Post to MR'}
                  </button>
                )}
                <button
                  className="diff-inline-edit"
                  onClick={() => ann.beginEdit(a)}
                  title="Edit this note before posting it"
                >
                  <Pencil size={11} strokeWidth={1.75} style={{ marginRight: 4 }} />
                  Edit
                </button>
                <button
                  className="diff-inline-resolve"
                  disabled={ann.resolvePending[a.id]}
                  onClick={() => ann.resolveNote(a.id)}
                  title="Mark this note resolved"
                >
                  <Check size={11} strokeWidth={2} style={{ marginRight: 4 }} />
                  {ann.resolvePending[a.id] ? 'Resolving…' : 'Resolve'}
                </button>
                <button
                  className="diff-inline-delete"
                  disabled={ann.deletePending[a.id]}
                  onClick={() => setConfirmDelete(true)}
                  title="Delete this annotation"
                >
                  <Trash2 size={11} strokeWidth={1.75} style={{ marginRight: 4 }} />
                  {ann.deletePending[a.id] ? 'Deleting…' : 'Delete'}
                </button>
              </>
            )}
          </div>
        </>
      )}
    </div>
  );
}

/**
 * The always-visible annotations under an annotated line. Rendered into a
 * CodeMirror block widget through a portal (see `InlineAnnotationsWidget`), and
 * carrying the same controls as the selected-line panel.
 */
export function InlineAnnotations({ anns, ann, mr }: { anns: Annotation[]; ann: AnnCtx; mr: Mr | null }) {
  return (
    <div className="diff-inline-block" onClick={(e) => e.stopPropagation()}>
      {anns.map((a) => (
        <AnnotationRow key={a.id} a={a} ann={ann} mr={mr} />
      ))}
    </div>
  );
}

/**
 * The inline annotation panel shown when a line/range is selected in either
 * editor: existing annotations for the range, MR threads (with reply), and the
 * comment form. Shared by the diff editor and the code editor so both surfaces
 * have an identical annotation experience, all driven by the `useAnnotations`
 * context (`AnnCtx`).
 */
export function AnnotationBlock({
  range, lineAnnotations, lineThreads, mr, ann, repoId, filePath, lineNum,
}: {
  range: LineRange;
  lineAnnotations: Annotation[];
  lineThreads: MrThread[];
  mr: Mr | null;
  ann: AnnCtx;
  repoId: string;
  filePath: string;
  lineNum: number;
}) {
  const multiline = range.startLine !== range.endLine;
  return (
    <div className="diff-inline-block" onClick={(e) => e.stopPropagation()}>
      {lineAnnotations.map((a) => (
        <AnnotationRow key={a.id} a={a} ann={ann} mr={mr} />
      ))}
      {lineThreads.map((d, i) => {
        const first = d.notes?.[0];
        const resolved = first?.resolved === true;
        const replyKey = `${repoId}/${filePath}/${lineNum}/${i}`;
        const noteCount = d.notes?.length ?? 0;
        const replyPending = ann.replyPending[replyKey] ?? false;
        return (
          <div key={d.id ?? i}>
            <div className={`diff-inline-thread ${resolved ? 'resolved' : ''}`}>
              <span className="diff-inline-author"
                style={{ color: resolved ? 'var(--gl-text-color-disabled)' : 'var(--gl-color-orange-400)' }}
              >
                ● {first?.author?.username ?? '?'}
              </span>
              <span className="diff-inline-content">{first?.body}</span>
              {noteCount > 1 && (
                <span className="diff-inline-replies">{noteCount - 1} repl{noteCount > 2 ? 'ies' : 'y'}</span>
              )}
            </div>
            {!resolved && mr && d.id && (
              <div className="diff-inline-reply-row">
                <input
                  className="diff-inline-reply-input"
                  placeholder="Respond… (Enter to send)"
                  value={ann.replyTexts[replyKey] ?? ''}
                  disabled={replyPending}
                  onChange={(e) => ann.setReplyTexts((prev) => ({ ...prev, [replyKey]: e.target.value }))}
                  onKeyDown={(e) => {
                    if (replyPending) return;
                    if (e.key === 'Enter') ann.submitReply(mr.id, d.id!, ann.replyTexts[replyKey] ?? '', replyKey);
                    if (e.key === 'Escape') ann.cancel();
                  }}
                />
                {(ann.replyTexts[replyKey] ?? '').trim() && (
                  <button className="diff-inline-reply-send btn-secondary"
                    disabled={replyPending}
                    onClick={() => ann.submitReply(mr.id, d.id!, ann.replyTexts[replyKey] ?? '', replyKey)}
                  >
                    Send
                  </button>
                )}
              </div>
            )}
          </div>
        );
      })}
      {lineThreads.length === 0 && (
        <div className="diff-inline-form">
          <div className="diff-inline-form-head">
            <MessageSquarePlus size={13} strokeWidth={1.75} />
            <span>
              {multiline
                ? `Commenting on lines ${range.startLine}–${range.endLine}`
                : `Commenting on line ${range.endLine}`}
            </span>
          </div>
          <textarea
            ref={ann.inputRef}
            className="diff-annotation-textarea"
            placeholder="Write a comment…  (⌘/Ctrl+Enter to save · Esc to cancel)"
            rows={3}
            value={ann.annotationText}
            onChange={(e) => ann.setAnnotationText(e.target.value)}
            onKeyDown={(e) => {
              if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') ann.submit();
              if (e.key === 'Escape') ann.cancel();
            }}
          />
          <div className="diff-inline-form-actions">
            <button
              className="btn-secondary diff-annotation-save"
              disabled={!ann.annotationText.trim()}
              onClick={ann.submit}
            >Comment</button>
            <button className="diff-inline-form-cancel" onClick={ann.cancel}>Cancel</button>
            <button
              className="diff-annotation-open-editor"
              onClick={() => ann.openInEditor(repoId, filePath, lineNum)}
              title="Open in editor at this line"
            >Open in editor ↗</button>
          </div>
        </div>
      )}
    </div>
  );
}
