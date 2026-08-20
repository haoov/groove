import { useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useStore, useSession } from '../shared/store';
import type { BlameLine } from '../shared/ipc/ipc';

/**
 * Per-line authorship for one file, fetched only while the blame gutter is on.
 *
 * `git blame` reads the file on disk, so an unsaved buffer keeps the attribution
 * git last saw. The cache is cleared with the diff cache, which covers commits and
 * external edits.
 */
export function useBlame(opts: {
  worktreeId: string | undefined;
  repoId: string;
  filePath: string;
}) {
  const { worktreeId, repoId, filePath } = opts;
  const key = `${repoId}/${filePath}`;
  const blameOn = useSession((s) => s.blameOn);
  const blame = useSession((s) => s.blameByFile[key]);
  const setBlame = useSession((s) => s.setBlame);
  const openTab = useSession((s) => s.openTab);
  const setLastError = useStore((s) => s.setLastError);
  const inFlight = useRef(false);

  useEffect(() => {
    if (!blameOn || !worktreeId || blame !== undefined || inFlight.current) return;
    inFlight.current = true;
    invoke<BlameLine[]>('blame_file', { worktreeId, filePath })
      .then((lines) => setBlame(key, lines))
      .catch((e) => setLastError(String(e)))
      .finally(() => { inFlight.current = false; });
  }, [blameOn, worktreeId, filePath, blame, key, setBlame, setLastError]);

  const openCommit = useCallback(
    (sha: string) => {
      openTab({ repoId, filePath: '', view: 'diff', kind: 'commit', sha, label: sha.slice(0, 7) });
    },
    [openTab, repoId],
  );

  return { blameOn, blame, openCommit };
}
