import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { GitPullRequest, Check } from 'lucide-react';
import { Markdown } from './Markdown';
import type { Mr, MrThread } from '../types/ipc';
import { openExternal } from '../lib/openExternal';
import { useSession } from '../store';

/** Group a raw CI status string for styling (forge-ci-* classes). */
export function ciGroup(status: string): 'ok' | 'fail' | 'run' | 'idle' {
  if (status === 'success') return 'ok';
  if (status === 'failed') return 'fail';
  if (['running', 'pending', 'preparing', 'created', 'waiting_for_resource', 'scheduled'].includes(status)) return 'run';
  return 'idle';
}

/** Review threads with reply counts + resolve flow. Rendered in the MR overview
 *  (moved out of the sidebar Forge section, which is now a compact list). */
export function MrThreadsSection({ threads, mr, onResolved }: { threads: MrThread[]; mr: Mr; onResolved: () => void }) {
  // A thread carries a diff position, so its header can open the file where the
  // comment is — the same affordance the annotation rows have.
  const openTab = useSession((s) => s.openTab);
  const worktrees = useSession((s) => s.activeWorktrees);
  const repoId = worktrees.find((w) => w.id === mr.worktree_id)?.repo_id ?? null;

  /** Where a note points, preferring the new side; a comment on a deleted line
   *  only has the old one. */
  const locate = (note: any): { path: string; line: number } | null => {
    const pos = note?.position;
    if (!pos) return null;
    if (pos.new_path && pos.new_line) return { path: pos.new_path, line: pos.new_line };
    if (pos.old_path && pos.old_line) return { path: pos.old_path, line: pos.old_line };
    return null;
  };

  // Keyed by thread id (stable across refetches), not array index — the thread
  // list is refetched after each resolve, so index-keyed state would smear onto
  // the wrong thread.
  const [expandedThreads, setExpandedThreads] = useState<Set<string>>(new Set());
  const [confirmingResolve, setConfirmingResolve] = useState<Set<string>>(new Set());
  const [resolving, setResolving] = useState<Set<string>>(new Set());
  const [resolveErrors, setResolveErrors] = useState<Record<string, string>>({});
  const resolvable = threads.filter((d: any) => d.notes?.[0]?.resolvable === true);
  if (!resolvable.length) return null;

  const toggleExpand = (id: string) =>
    setExpandedThreads((prev) => {
      const n = new Set(prev);
      if (n.has(id)) n.delete(id); else n.add(id);
      return n;
    });

  const confirmResolve = (id: string) =>
    setConfirmingResolve((prev) => { const n = new Set(prev); n.add(id); return n; });

  const cancelResolve = (id: string) =>
    setConfirmingResolve((prev) => { const n = new Set(prev); n.delete(id); return n; });

  const doResolve = async (id: string, threadId: string) => {
    setResolving((prev) => { const n = new Set(prev); n.add(id); return n; });
    setResolveErrors((prev) => { const n = { ...prev }; delete n[id]; return n; });
    try {
      await invoke('resolve_mr_thread', { mrId: mr.id, threadId });
      await onResolved();
    } catch (e) {
      setResolveErrors((prev) => ({ ...prev, [id]: String(e) }));
    } finally {
      setResolving((prev) => { const n = new Set(prev); n.delete(id); return n; });
      setConfirmingResolve((prev) => { const n = new Set(prev); n.delete(id); return n; });
    }
  };

  return (
    <div className="mr-threads-section">
      <div className="mr-threads-header">
        <GitPullRequest size={11} strokeWidth={1.75} style={{ marginRight: 5, verticalAlign: 'middle' }} />
        MR threads
        <a
          className="mr-threads-link"
          href={mr.url}
          onClick={(e) => { e.preventDefault(); openExternal(mr.url); }}
        >
          #{mr.remote_id}
        </a>
      </div>
      {resolvable.map((d: any, i: number) => {
        const key: string = d.id ?? `idx-${i}`;
        const first = d.notes[0];
        const replies = d.notes.slice(1);
        const resolved: boolean = first.resolved === true;
        const expanded = expandedThreads.has(key);
        const filePart = first.position?.new_path?.split('/').pop() ?? null;
        const linePart = first.position?.new_line ? `:${first.position.new_line}` : null;
        return (
          <div key={key} className={`mr-thread-item ${resolved ? 'mr-thread-resolved' : ''}`}>
            <button
              className="annotation-loc"
              disabled={!repoId || !locate(first)}
              title={locate(first) ? `Open ${locate(first)!.path}:${locate(first)!.line}` : undefined}
              onClick={() => {
                const at = locate(first);
                if (!at || !repoId) return;
                openTab({ repoId, filePath: at.path, view: 'edit', cursorLine: at.line });
              }}
            >
              <span className="thread-loc-author">{first.author?.username ?? '?'}</span>
              {filePart
                ? <><span className="annotation-file">{filePart}</span><span className="annotation-line">{linePart}</span></>
                : <span className="annotation-file">#{mr.remote_id}</span>
              }
              <span
                className="annotation-author-dot"
                style={{ color: resolved ? 'var(--gl-color-green-400)' : 'var(--gl-color-orange-400)' }}
              >●</span>
            </button>
            <div className="annotation-content mr-thread-body">
              <Markdown text={first.body} />
            </div>
            <div className="annotation-meta">
              {replies.length > 0 ? (
                <button className="mr-thread-replies-toggle" onClick={() => toggleExpand(key)}>
                  {expanded ? '▾' : '▸'} {replies.length} repl{replies.length > 1 ? 'ies' : 'y'}
                </button>
              ) : <span />}
              {!resolved && (
                confirmingResolve.has(key) ? (
                  <div className="mr-thread-resolve-confirm">
                    <span className="mr-thread-resolve-prompt">Resolve thread?</span>
                    <button
                      className="annotation-resolve"
                      disabled={resolving.has(key)}
                      onClick={() => doResolve(key, d.id)}
                    >
                      {resolving.has(key) ? '…' : '✓ Yes'}
                    </button>
                    <button className="mr-thread-resolve-cancel" onClick={() => cancelResolve(key)}>✕</button>
                  </div>
                ) : (
                  <button className="annotation-resolve" onClick={() => confirmResolve(key)}>
                    <Check size={12} strokeWidth={2} style={{ marginRight: 4, verticalAlign: 'middle' }} />
                    Resolve
                  </button>
                )
              )}
            </div>
            {resolveErrors[key] && (
              <div className="mr-thread-resolve-error">{resolveErrors[key]}</div>
            )}
            {expanded && replies.length > 0 && (
              <div className="mr-thread-replies">
                {replies.map((r: any, j: number) => (
                  <div key={j} className="mr-thread-reply">
                    <span className="mr-thread-author">{r.author?.username ?? '?'}</span>
                    <div className="mr-thread-body">
                      <Markdown text={r.body} />
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
