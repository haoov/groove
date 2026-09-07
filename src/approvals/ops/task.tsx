import { CheckCircle2, Clock, FilePlus, FileText, FolderPlus, GitBranch, Tag } from 'lucide-react';
import { OP } from '../../shared/ipc/ops';
import { EditableField, Field } from './fields';
import { str, type Edits, type OpViewProps, type OpsOf, type Payload } from './spec';

const seedDraft = (p: Payload): Edits => ({
  title: str(p, 'title'),
  body_markdown: str(p, 'body_markdown'),
});

function Draft({ edits, setField }: Omit<OpViewProps, 'payload'>) {
  return (
    <>
      <EditableField
        label="Title"
        value={edits.title ?? ''}
        onChange={(v) => setField('title', v)}
        placeholder="Task title"
      />
      <EditableField
        label="Description"
        value={edits.body_markdown ?? ''}
        onChange={(v) => setField('body_markdown', v)}
        multiline
        placeholder="Task description (markdown)"
      />
    </>
  );
}

export const TASK_OPS: OpsOf<'task.'> = {
  [OP.TASK_PROPERTY]: {
    label: 'Update task property',
    icon: Tag,
    View: ({ payload }) => (
      <>
        <Field label="Property" value={str(payload, 'property')} />
        <Field label="Value" value={typeof payload.value === 'string' ? payload.value : JSON.stringify(payload.value)} mono />
        <Field label="Task" value={str(payload, 'task_id')} mono />
      </>
    ),
  },

  [OP.TASK_HOURS]: {
    label: 'Log hours',
    icon: Clock,
    View: ({ payload }) => (
      <>
        <Field label="Hours" value={String(payload.hours ?? '')} mono />
        <Field label="Task" value={str(payload, 'task_id')} mono />
      </>
    ),
  },

  [OP.TASK_BODY]: {
    label: 'Update task description',
    icon: FileText,
    seed: (p) => ({ markdown: str(p, 'markdown') }),
    View: ({ payload, edits, setField }) => (
      <>
        <Field label="Task" value={str(payload, 'task_id')} mono />
        <EditableField
          label="New description"
          value={edits.markdown ?? ''}
          onChange={(v) => setField('markdown', v)}
          multiline
          placeholder="Page body (markdown)"
        />
        <div className="cp-hint cp-hint--danger">
          Replaces the whole task body. Anything markdown cannot represent is lost.
        </div>
      </>
    ),
  },

  [OP.TASK_CREATE]: {
    label: 'Create task',
    icon: FilePlus,
    seed: seedDraft,
    View: Draft,
  },

  [OP.TASK_CREATE_FROM_EXPLORER]: {
    label: 'Create task from explorer',
    icon: FilePlus,
    seed: seedDraft,
    View: ({ payload, edits, setField }) => (
      <>
        <Draft edits={edits} setField={setField} />
        {/* The source session must be visible before approval. */}
        <Field label="Converting" value={str(payload, 'explorer_id')} mono />
        <div className="cp-hint">
          Files a task, then moves this session&apos;s worktrees, repos, and annotations onto it.
        </div>
      </>
    ),
  },

  [OP.TASK_ADD_REPO]: {
    label: 'Add repo to task',
    icon: FolderPlus,
    View: ({ payload }) => (
      <>
        <Field label="Repo" value={str(payload, 'repo')} mono />
        <Field label="Task" value={str(payload, 'task_id')} mono />
        {str(payload, 'branch') && <Field label="Branch" value={str(payload, 'branch')} mono />}
        {str(payload, 'target_branch') && <Field label="Based on" value={str(payload, 'target_branch')} mono />}
        <div className="cp-hint">
          Attaches the repo and creates its worktree. Repos already on the task stay.
        </div>
      </>
    ),
  },

  [OP.TASK_ADD_WORKTREE]: {
    label: 'Add worktree to task',
    icon: GitBranch,
    View: ({ payload }) => (
      <>
        <Field label="Repo"   value={str(payload, 'repo') || 'the task’s only repo'} mono />
        <Field label="Task"   value={str(payload, 'task_id')} mono />
        <Field label="Branch" value={str(payload, 'branch')} mono />
        {str(payload, 'target_branch') && <Field label="Based on" value={str(payload, 'target_branch')} mono />}
        <div className="cp-hint">
          Checks out another branch of a repo the task already has, beside the
          worktrees it holds now. Nothing existing is touched.
        </div>
      </>
    ),
  },

  [OP.TASK_FINISH]: {
    label: 'Finish task',
    icon: CheckCircle2,
    View: ({ payload }) => (
      <>
        <Field label="Task" value={str(payload, 'task_id')} mono />
        <div className="cp-hint cp-hint--danger">
          Marks the task done at its source, then removes every worktree of this
          session and its local data. Anything not committed and pushed is lost —
          this cannot be undone.
        </div>
      </>
    ),
  },
};
