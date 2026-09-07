import { GitPullRequest, RefreshCw, X } from 'lucide-react';
import { OP } from '../../shared/ipc/ops';
import { EditableField, Field } from './fields';
import { repoName, str, type Edits, type OpViewProps, type OpsOf, type Payload } from './spec';

/** Mirrors the headings the create_mr contract requires (MR_DESCRIPTION in definitions.rs). */
const MR_SKELETON = '## What\n\n\n## Why\n\n';

const seedText = (p: Payload): Edits => ({
  title: str(p, 'title'),
  description: str(p, 'description') || MR_SKELETON,
});

function MrText({ edits, setField }: Omit<OpViewProps, 'payload'>) {
  return (
    <>
      <EditableField
        label="Title"
        value={edits.title ?? ''}
        onChange={(v) => setField('title', v)}
        placeholder="Merge request title"
      />
      <EditableField
        label="Description"
        value={edits.description ?? ''}
        onChange={(v) => setField('description', v)}
        multiline
        placeholder="Describe the change…"
      />
    </>
  );
}

export const MR_OPS: OpsOf<'mr.'> = {
  [OP.MR_CREATE]: {
    label: 'Create MR',
    icon: GitPullRequest,
    seed: seedText,
    // create_mr_impl falls back to a "WIP" title on an empty field.
    validate: (edits) => ((edits.title ?? '').trim() ? null : 'Give the merge request a title first.'),
    View: ({ payload, edits, setField }) => (
      <>
        <Field label="Repo"   value={repoName(payload)} />
        <Field label="Branch" value={str(payload, 'branch')} mono />
        <Field label="Into"   value={str(payload, 'target_branch')} mono />
        <MrText edits={edits} setField={setField} />
      </>
    ),
  },

  [OP.MR_UPDATE]: {
    label: 'Update MR',
    icon: RefreshCw,
    seed: seedText,
    View: MrText,
  },

  [OP.MR_CLOSE]: {
    label: 'Close MR',
    icon: X,
    View: () => <div className="cp-hint">This will close the MR and cannot be undone.</div>,
  },
};
