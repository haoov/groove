import { ChevronsUp, Download, GitCommit, RotateCcw, Upload } from 'lucide-react';
import { OP } from '../../shared/ipc/ops';
import { EditableField, Field } from './fields';
import { repoName, str, type OpsOf } from './spec';

/** Repo and branch, the head of every git payload. */
function Target({ payload }: { payload: Record<string, unknown> }) {
  return (
    <>
      <Field label="Repo"   value={repoName(payload)} />
      <Field label="Branch" value={str(payload, 'branch')} mono />
    </>
  );
}

export const GIT_OPS: OpsOf<'git.'> = {
  [OP.GIT_COMMIT]: {
    label: 'Git commit',
    icon: GitCommit,
    seed: (p) => ({ message: str(p, 'message') }),
    View: ({ payload, edits, setField }) => (
      <>
        <Target payload={payload} />
        <EditableField
          label="Message"
          value={edits.message ?? ''}
          onChange={(v) => setField('message', v)}
          multiline
          placeholder="Commit message (first line is the title)"
        />
      </>
    ),
  },

  [OP.GIT_PUSH]: {
    label: 'Git push',
    icon: Upload,
    View: ({ payload }) => (
      <>
        <Target payload={payload} />
        <div className="cp-hint">Pushes local commits to origin.</div>
      </>
    ),
  },

  [OP.GIT_PULL]: {
    label: 'Git pull',
    icon: Download,
    View: ({ payload }) => (
      <>
        <Target payload={payload} />
        <div className="cp-hint">Pulls and rebases from origin.</div>
      </>
    ),
  },

  [OP.GIT_REBASE]: {
    label: 'Rebase on main',
    icon: ChevronsUp,
    View: ({ payload }) => (
      <>
        <Target payload={payload} />
        <Field label="Onto" value={`origin/${str(payload, 'default_branch') || 'main'}`} mono />
      </>
    ),
  },

  [OP.GIT_DISCARD]: {
    label: 'Discard changes',
    icon: RotateCcw,
    View: ({ payload }) => (
      <>
        <Field label="File" value={str(payload, 'file_path')} mono />
        <div className="cp-hint cp-hint--danger">Permanently discards this file&apos;s local changes — this cannot be undone.</div>
      </>
    ),
  },

  [OP.GIT_DISCARD_ALL]: {
    label: 'Discard all changes',
    icon: RotateCcw,
    View: ({ payload }) => (
      <>
        <Field label="Repo" value={repoName(payload)} />
        <div className="cp-hint cp-hint--danger">Permanently discards ALL local changes (reverts tracked files and removes untracked ones) — this cannot be undone.</div>
      </>
    ),
  },
};
