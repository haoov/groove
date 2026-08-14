import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useStore } from '../store';
import { mergeExpansion, stepRange, type Gap } from '../lib/diffGaps';
import type { Hunk } from '../types/ipc';

interface FileLines {
  lines: string[];
  total: number;
}

/**
 * Fills the unshown stretches of one file's diff on demand.
 *
 * The fetched lines are merged back into the hunk list and handed to `onHunks`, so
 * the editor only ever sees hunks. `rev` is the commit for a commit diff; leave it
 * undefined for the three working-tree modes, whose new side is the file on disk.
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
  // A click is async: read the hunks and the writer as they are when it resolves.
  const hunksRef = useRef(hunks);
  hunksRef.current = hunks;
  const applyRef = useRef(onHunks);
  applyRef.current = onHunks;

  const empty = hunks === undefined || hunks.length === 0;

  // One metadata probe per file: `end: 0` asks for no lines and just reports the
  // length, which is what decides whether a trailing gap exists.
  useEffect(() => {
    setTotal(undefined);
    if (!worktreeId || empty) return;
    let live = true;
    invoke<FileLines>('read_file_lines', {
      worktreeId, filePath, start: 1, end: 0, rev: rev ?? null,
    })
      .then((r) => { if (live) setTotal(r.total); })
      .catch(() => { /* gaps between hunks still expand; only the tail is lost */ });
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
