import { useCallback, useEffect, useState } from 'react';
import { call } from '../shared/ipc/client';
import { useStore } from '../shared/store';
import type { Repo } from '../shared/ipc/generated';

// list_main_repos / clone_repo return MainRepo (no ts-rs DTO): { local_path, slug }.
interface MainRepo { local_path: string; slug: string }

/** Add repos to the active session, then provision worktrees. Repos come from
 *  the local clone pool (or a fresh clone); each is registered, attached to the
 *  session, and provisioned. A reopen refreshes the workspace. */
export function AddRepoModal() {
  const open = useStore((s) => s.addRepoOpen);
  const setOpen = useStore((s) => s.setAddRepoOpen);
  const activeId = useStore((s) => s.activeSessionId);
  const session = useStore((s) => (activeId ? s.sessions[activeId] : undefined));

  const [pool, setPool] = useState<MainRepo[] | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [cloneUrl, setCloneUrl] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const attachedSlugs = new Set((session?.repos ?? []).map((r) => r.id));

  const load = useCallback(() => {
    call<MainRepo[]>('list_main_repos').then(setPool).catch((e) => { setError(String(e)); setPool([]); });
  }, []);
  useEffect(() => { if (open) { setPicked(new Set()); setError(null); load(); } }, [open, load]);

  if (!open || !session) return null;

  const toggle = (slug: string) =>
    setPicked((s) => { const n = new Set(s); n.has(slug) ? n.delete(slug) : n.add(slug); return n; });

  const clone = async () => {
    if (!cloneUrl.trim()) return;
    setBusy(true); setError(null);
    try { const mr = await call<MainRepo>('clone_repo', { url: cloneUrl.trim() }); setCloneUrl(''); load(); toggle(mr.slug); }
    catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  };

  const add = async () => {
    const chosen = (pool ?? []).filter((r) => picked.has(r.slug));
    if (chosen.length === 0) return;
    setBusy(true); setError(null);
    try {
      // Register each pool entry into the DB to get a Repo id.
      const newRepos = await Promise.all(
        chosen.map((r) => call<Repo>('register_repo', { slug: r.slug, localPath: r.local_path })),
      );
      const newIds = newRepos.map((r) => r.id);
      // set_task_repos REPLACES the set, so merge with what's already attached.
      const merged = Array.from(new Set([...session.repos.map((r) => r.id), ...newIds]));
      await call('set_task_repos', { shortId: session.id, repoIds: merged });
      // Session-default branch (null) → task branch, or explorer/<slug>.
      await call('provision_worktrees', {
        taskId: session.id,
        branches: newIds.map((id) => ({ repo_id: id, branch_name: null })),
      });
      // Reopen to re-hydrate the workspace via workspace_ready.
      await call('open_task', { shortId: session.id }).catch(() => {});
      setOpen(false);
    } catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="ov-scrim" onMouseDown={(e) => { if (e.target === e.currentTarget) setOpen(false); }}>
      <div className="addrepo" role="dialog" aria-modal="true">
        <div className="settings-h"><span>Add repo to {session.title}</span><button className="settings-x" onClick={() => setOpen(false)}>×</button></div>

        <div className="ar-clone">
          <input placeholder="Clone a new repo by URL…" value={cloneUrl} inputMode="url"
            onChange={(e) => setCloneUrl(e.target.value)} onKeyDown={(e) => { if (e.key === 'Enter') clone(); }} />
          <button disabled={busy || !cloneUrl.trim()} onClick={clone}>Clone</button>
        </div>

        <div className="ar-list">
          {pool === null && <div className="ar-empty">Loading…</div>}
          {pool?.length === 0 && <div className="ar-empty">No repos in the pool. Clone one above.</div>}
          {pool?.map((r) => {
            const already = attachedSlugs.has(r.slug);
            return (
              <label key={r.slug} className={`ar-row${already ? ' disabled' : ''}`}>
                <input type="checkbox" disabled={already} checked={already || picked.has(r.slug)} onChange={() => toggle(r.slug)} />
                <span className="ar-slug">{r.slug}</span>
                {already && <span className="ar-tag">attached</span>}
              </label>
            );
          })}
        </div>

        {error && <div className="fr-error">{error}</div>}
        <div className="ar-actions">
          <button className="ovw-ghost" onClick={() => setOpen(false)}>Cancel</button>
          <button className="fr-save" disabled={busy || picked.size === 0} onClick={add}>
            {busy ? 'Adding…' : `Add ${picked.size || ''}`}
          </button>
        </div>
      </div>
    </div>
  );
}
