import { useEffect, useMemo, useState } from 'react';
import { call } from '../shared/ipc/client';
import { useStore } from '../shared/store';
import { activeWorktree } from '../sessions/sessions.slice';
import type { SessionState } from '../sessions/sessions.slice';

interface Dir { name: string; dirs: Map<string, Dir>; files: string[]; }

/** Fold flat relative paths into a directory tree. */
function buildTree(paths: string[]): Dir {
  const root: Dir = { name: '', dirs: new Map(), files: [] };
  for (const p of paths) {
    const parts = p.split('/');
    let node = root;
    for (let i = 0; i < parts.length - 1; i++) {
      const seg = parts[i];
      if (!node.dirs.has(seg)) node.dirs.set(seg, { name: seg, dirs: new Map(), files: [] });
      node = node.dirs.get(seg)!;
    }
    node.files.push(parts[parts.length - 1]);
  }
  return root;
}

function TreeDir({
  dir, prefix, depth, activePath, onOpen,
}: {
  dir: Dir; prefix: string; depth: number; activePath: string | null; onOpen: (p: string) => void;
}) {
  const [open, setOpen] = useState(depth < 1);
  const dirs = [...dir.dirs.values()].sort((a, b) => a.name.localeCompare(b.name));
  const files = [...dir.files].sort((a, b) => a.localeCompare(b));
  return (
    <>
      {depth >= 0 && (
        <div className="tnode tdir" style={{ paddingLeft: 8 + depth * 12 }} onClick={() => setOpen((o) => !o)}>
          <span className="tcaret">{open ? '▾' : '▸'}</span>{dir.name}
        </div>
      )}
      {open && (
        <>
          {dirs.map((d) => (
            <TreeDir key={prefix + d.name} dir={d} prefix={`${prefix}${d.name}/`} depth={depth + 1} activePath={activePath} onOpen={onOpen} />
          ))}
          {files.map((f) => {
            const full = prefix + f;
            return (
              <div
                key={full}
                className={`tnode tfile${activePath === full ? ' on' : ''}`}
                style={{ paddingLeft: 8 + (depth + 1) * 12 }}
                onClick={() => onOpen(full)}
              >
                {f}
              </div>
            );
          })}
        </>
      )}
    </>
  );
}

export function FilesPanel({ session }: { session: SessionState }) {
  const openFileTab = useStore((s) => s.openFileTab);
  const wt = activeWorktree(session);
  const [paths, setPaths] = useState<string[] | null>(null);
  const [q, setQ] = useState('');

  useEffect(() => {
    setPaths(null);
    if (!wt) return;
    call<string[]>('list_files', { worktreePath: wt.path })
      .then(setPaths)
      .catch((e) => { console.warn('list_files failed', e); setPaths([]); });
  }, [wt?.path]);

  const tree = useMemo(() => buildTree(paths ?? []), [paths]);
  const activePath = session.tabs.find((t) => t.id === session.activeTabId)?.path ?? null;
  const open = (p: string) => wt && openFileTab(session.id, wt.repo_id, p);

  const filtered = q.trim()
    ? (paths ?? []).filter((p) => p.toLowerCase().includes(q.trim().toLowerCase()))
    : null;

  return (
    <aside className="sidebar">
      <div className="side-h">Files</div>
      <div className="side-search">
        <input placeholder="Filter files…" value={q} onChange={(e) => setQ(e.target.value)} />
      </div>
      <div className="tree2">
        {!wt && <div className="tempty">No worktree in this session.</div>}
        {wt && paths === null && <div className="tempty">Loading…</div>}
        {wt && paths?.length === 0 && <div className="tempty">Empty worktree.</div>}
        {filtered
          ? filtered.map((p) => (
              <div key={p} className={`tnode tfile${activePath === p ? ' on' : ''}`} style={{ paddingLeft: 12 }} onClick={() => open(p)}>
                {p}
              </div>
            ))
          : paths && paths.length > 0 && (
              <TreeDir dir={tree} prefix="" depth={-1} activePath={activePath} onOpen={open} />
            )}
      </div>
    </aside>
  );
}
