import { useState } from 'react';
import { Check, MessageSquare, Send, Trash2 } from 'lucide-react';
import { Markdown } from '../Markdown';
import type { Annotation, Mr, Repo } from '../../types/ipc';

export function AnnotationsTab({
  annotations, repoFor, onResolve, onDelete, onOpen, mr, onPostToMr,
}: {
  annotations: Annotation[];
  repoFor: (id: string) => Repo | undefined;
  onResolve: (id: string) => void;
  /** Remove the note outright — for a wrong line or a bad agent call. */
  onDelete: (id: string) => void;
  /** Jump to the annotated line in the editor. */
  onOpen: (a: Annotation) => void;
  /** The repo's MR, when one exists — enables "Post to MR" per annotation. */
  mr?: Mr | null;
  onPostToMr?: (a: Annotation) => Promise<void>;
}) {
  const [posting, setPosting] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const open = annotations.filter((a) => a.status === 'open');
  if (open.length === 0) return <div className="sidebar-empty">No open annotations</div>;

  const post = async (a: Annotation) => {
    if (!onPostToMr || posting) return;
    setPosting(a.id);
    try {
      await onPostToMr(a);
    } finally {
      setPosting(null);
    }
  };

  return (
    <div className="annotations-list">
      {open.map((a) => (
        <div key={a.id} className="annotation-item">
          {/* The location line is the affordance: it opens the file at the line. */}
          <button className="annotation-loc" onClick={() => onOpen(a)} title="Open at this line">
            <MessageSquare
              size={12}
              strokeWidth={1.75}
              className="annotation-icon"
              style={{ color: a.author === 'agent' ? 'var(--wb-annotation-agent)' : 'var(--wb-annotation-human)' }}
            />
            <span className="annotation-file">{a.file_path.split('/').pop()}</span>
            <span className="annotation-line">
              :{a.start_line !== a.end_line ? `${a.start_line}–${a.end_line}` : a.line_num}
            </span>
          </button>
          <div className="annotation-content">
            <Markdown text={a.content} />
          </div>
          <div className="annotation-meta">
            {repoFor(a.repo_id)?.project ?? a.repo_id}
            {confirmDelete === a.id ? (
              <span className="annotation-actions">
                <span className="annotation-confirm">delete?</span>
                <button className="annotation-resolve annotation-delete" onClick={() => { setConfirmDelete(null); onDelete(a.id); }}>
                  yes
                </button>
                <button className="annotation-resolve" onClick={() => setConfirmDelete(null)}>no</button>
              </span>
            ) : (
              <span className="annotation-actions">
                {mr && onPostToMr && (
                  <button
                    className="annotation-resolve annotation-post-mr"
                    disabled={posting === a.id}
                    onClick={() => post(a)}
                    title="Publish as an MR discussion at its line (resolves the local annotation)"
                  >
                    <Send size={12} strokeWidth={1.75} style={{ marginRight: 4, verticalAlign: 'middle' }} />
                    {posting === a.id ? 'Posting…' : 'Post to MR'}
                  </button>
                )}
                <button className="annotation-resolve" onClick={() => onResolve(a.id)}>
                  <Check size={12} strokeWidth={2} style={{ marginRight: 4, verticalAlign: 'middle' }} />
                  Resolve
                </button>
                <button
                  className="annotation-resolve annotation-delete"
                  onClick={() => setConfirmDelete(a.id)}
                  title="Delete this annotation"
                >
                  <Trash2 size={12} strokeWidth={1.75} />
                </button>
              </span>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
