import { GitCompare, Cloud, Pencil, type LucideIcon } from 'lucide-react';
import type { DiffMode } from '../store';

/** The diff comparison-base options, shared by the sidebar Changes toolbar and
 *  the command palette. */
export const DIFF_MODES: { id: DiffMode; label: string; title: string; Icon: LucideIcon }[] = [
  { id: 'vs-main',   label: 'main',    title: 'Diff vs the default branch (origin/main)',          Icon: GitCompare },
  { id: 'vs-remote', label: 'remote',  title: "Diff vs this branch's remote — unpushed local work", Icon: Cloud },
  { id: 'working',   label: 'working', title: 'Uncommitted working-tree changes (vs HEAD)',         Icon: Pencil },
];
