import { Sparkles } from 'lucide-react';
import { OP } from '../../shared/ipc/ops';
import { EditableField, Field } from './fields';
import { str, type OpsOf } from './spec';

export const SKILL_OPS: OpsOf<'skill.'> = {
  [OP.SKILL_SAVE]: {
    label: 'Write agent skill',
    icon: Sparkles,
    seed: (p) => ({ body: str(p, 'body') }),
    View: ({ payload, edits, setField }) => (
      <>
        <Field label="Skill" value={`user:${str(payload, 'name')}`} mono />
        {str(payload, 'previous') && <Field label="Replaces" value={`user:${str(payload, 'previous')}`} mono />}
        <EditableField
          label="SKILL.md"
          value={edits.body ?? ''}
          onChange={(v) => setField('body', v)}
          multiline
          placeholder="Front matter, then the procedure"
        />
        <div className="cp-hint">
          Agents load skills at startup — this one works after a reload.
        </div>
      </>
    ),
  },
};
