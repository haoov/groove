import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { mergeExpansion, stepRange, type Gap } from '../shared/lib/diffGaps';
import type { Hunk, FileLines } from '../shared/ipc/ipc';



/**
 * Fills the hidden stretches of one file's diff on demand and hands merged hunks to `onHunks`.
 * `rev` is the commit for a commit diff; undefined for the working-tree modes.
 */
export function useDiffExpand(opts: {
  worktreeId: string | undefined;
  filePath: string;
  hunks: Hunk[] | undefined;
  onHunks: (hunks: Hunk[]) => void;
  rev?: string;
}) {
  const { worktreeId, filePath, hunks, onHunks, rev } = opts;
  const setLastError = useStore((s) => s.setLastError);
  const [total, setTotal] = useState<number | undefined>(undefined);
  const busy = useRef(false);
  // Read when the click resolves, not when it starts.
  const hunksRef = useRef(hunks);
  hunksRef.current = hunks;
  const applyRef = useRef(onHunks);
  applyRef.current = onHunks;

  const empty = hunks === undefined || hunks.length === 0;

  // `end: 0` fetches no lines and reports the file length.
  useEffect(() => {
    setTotal(undefined);
    if (!worktreeId || empty) return;
    let live = true;
    invoke<FileLines>('read_file_lines', {
      worktreeId, filePath, start: 1, end: 0, rev: rev ?? null,
    })
      .then((r) => { if (live) setTotal(r.total); })
      .catch(() => { /* only the trailing gap is lost */ });
    return () => { live = false; };
  }, [worktreeId, filePath, rev, empty]);

  const onExpand = useCallback(
    async (gap: Gap, whole: boolean) => {
      if (!worktreeId || busy.current) return;
      const { start, end } = stepRange(gap, whole);
      busy.current = true;
      try {
        const r = await invoke<FileLines>('read_file_lines', {
          worktreeId, filePath, start, end, rev: rev ?? null,
        });
        setTotal(r.total);
        const current = hunksRef.current;
        if (current) applyRef.current(mergeExpansion(current, gap, start, r.lines));
      } catch (e) {
        setLastError(String(e));
      } finally {
        busy.current = false;
      }
    },
    [worktreeId, filePath, rev, setLastError],
  );

  return { total, onExpand };
}
