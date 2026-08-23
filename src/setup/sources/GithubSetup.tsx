import { useEffect, useState } from 'react';
import { invoke } from '../../shared/ipc/invoke';
import type { DetectedSchema, GithubProject } from '../../shared/ipc/ipc';
import { DetectedPanel } from './DetectedPanel';

export interface GithubDraft {
  projectId: string;
  projectTitle: string;
  /** Detected from the board, and what the app will write. Empty until a board
   *  is picked and read. */
  statusMap: { ready: string; in_progress: string; done: string };
}

/** Nothing is typed in the happy path: gh already holds the credential, so the
 *  form is a list of the boards it can see. */
export function GithubSetup({
  value, onChange, onNeedsScope,
}: {
  value: GithubDraft;
  onChange: (v: GithubDraft) => void;
  /** A missing scope is not a form error — it is fixed in the tools list above. */
  onNeedsScope: (scope: string) => void;
}) {
  const [projects, setProjects] = useState<GithubProject[] | null>(null);
  const [detected, setDetected] = useState<DetectedSchema | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setBusy(true);
    setError(null);
    try {
      setProjects(await invoke<GithubProject[]>('list_github_projects', {}));
    } catch (e) {
      const msg = String(e);
      // GitHub names the scope it wanted; pass it on rather than paraphrasing.
      const scope = /required scopes: \['([^']+)'\]/.exec(msg)?.[1];
      if (scope) onNeedsScope(scope);
      setError(msg);
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    if (!value.projectId) { setDetected(null); return; }
    invoke<DetectedSchema>('detect_github_project', { projectId: value.projectId })
      .then((d) => {
        setDetected(d);
        onChange({
          ...value,
          statusMap: {
            ready: d.status_ready,
            in_progress: d.status_in_progress,
            done: d.status_done,
          },
        });
      })
      .catch((e) => setError(String(e)));
    // Only re-read when the board changes; onChange is the reporting channel.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value.projectId]);

  return (
    <>
      <label className="firstrun-field">
        <span className="firstrun-label">Board</span>
        {projects === null ? (
          <button type="button" className="btn-secondary" onClick={load} disabled={busy}>
            {busy ? 'Looking…' : 'Find my boards'}
          </button>
        ) : (
          <select
            className="firstrun-input"
            value={value.projectId}
            onChange={(e) => {
              const p = projects.find((x) => x.id === e.target.value);
              onChange({
                projectId: e.target.value,
                projectTitle: p?.title ?? '',
                statusMap: { ready: '', in_progress: '', done: '' },
              });
            }}
          >
            <option value="">Pick a board…</option>
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.owner}/{p.title} #{p.number}{p.closed ? ' (closed)' : ''}
              </option>
            ))}
          </select>
        )}
        <span className="firstrun-hint">
          Issues assigned to you that sit on this board become tasks. An issue that is
          on no board is not a task.
        </span>
      </label>

      {error && <div className="firstrun-warn"><span>{error}</span></div>}

      {detected && (
        <DetectedPanel
          detected={detected}
          note="Read from the board's own fields."
        />
      )}
    </>
  );
}
