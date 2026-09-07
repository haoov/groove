// ── The approval-op registry ──────────────────────────────────────────────────
// One OpSpec per OpType; ConfirmModal is the dialog shell around it.

import type { OpType } from '../../shared/ipc/ops';
import { GIT_OPS } from './git';
import { MR_OPS } from './mr';
import { TASK_OPS } from './task';
import { SKILL_OPS } from './skill';
import type { OpSpec } from './spec';

export const OP_SPECS: Record<OpType, OpSpec> = {
  ...GIT_OPS,
  ...MR_OPS,
  ...TASK_OPS,
  ...SKILL_OPS,
};

/** The spec for an op_type the backend sent, or undefined when the frontend is older. */
export const opSpec = (opType: string): OpSpec | undefined => OP_SPECS[opType as OpType];
