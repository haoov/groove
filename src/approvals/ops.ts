// Per-op presentation for the confirmation modal. The op_type strings mirror
// approvals/ops.rs; payload fields are whatever that op posted to the bridge.

type Payload = Record<string, unknown>;

const str = (p: Payload, k: string): string => {
  const v = p[k];
  return typeof v === 'string' ? v : v == null ? '' : String(v);
};

export interface EditField {
  key: string;
  label: string;
  multiline?: boolean;
}

export interface OpMeta {
  title: string;
  /** Approve-button label. */
  verb: string;
  danger?: boolean;
  edit?: EditField[];
  /** Label → value rows shown as the request detail. */
  summary: (p: Payload) => [string, string][];
}

const OPS: Record<string, OpMeta> = {
  'git.commit': {
    title: 'Commit changes', verb: 'Commit',
    edit: [{ key: 'message', label: 'Commit message', multiline: true }],
    summary: (p) => [['Branch', str(p, 'branch')]],
  },
  'git.push': {
    title: 'Push branch', verb: 'Push',
    summary: (p) => [['Branch', str(p, 'branch')], ['Remote', 'origin']],
  },
  'git.pull': {
    title: 'Pull branch', verb: 'Pull',
    summary: (p) => [['Branch', str(p, 'branch')], ['Mode', 'rebase']],
  },
  'git.rebase': {
    title: 'Rebase onto base', verb: 'Rebase',
    summary: (p) => [['Branch', str(p, 'branch')], ['Onto', str(p, 'default_branch') || 'main']],
  },
  'git.discard': {
    title: 'Discard file changes', verb: 'Discard', danger: true,
    summary: (p) => [['File', str(p, 'file_path')]],
  },
  'git.discard_all': {
    title: 'Discard ALL changes', verb: 'Discard all', danger: true,
    summary: () => [['Scope', 'Every local change in this worktree']],
  },
  'mr.create': {
    title: 'Open merge request', verb: 'Create MR',
    edit: [
      { key: 'title', label: 'Title' },
      { key: 'description', label: 'Description', multiline: true },
    ],
    summary: (p) => [['Source', str(p, 'source_branch')], ['Target', str(p, 'target_branch')]],
  },
  'mr.update': {
    title: 'Update merge request', verb: 'Update MR',
    edit: [
      { key: 'title', label: 'Title' },
      { key: 'description', label: 'Description', multiline: true },
    ],
    summary: (p) => [['MR', str(p, 'iid') ? `!${str(p, 'iid')}` : str(p, 'project_full')]],
  },
  'mr.close': {
    title: 'Close merge request', verb: 'Close MR', danger: true,
    summary: (p) => [['MR', str(p, 'iid') ? `!${str(p, 'iid')}` : str(p, 'project_full')]],
  },
  'notion.property': {
    title: 'Update Notion property', verb: 'Update',
    summary: (p) => [['Property', str(p, 'property')], ['Value', str(p, 'value')]],
  },
  'notion.hours': {
    title: 'Log hours to Notion', verb: 'Log hours',
    summary: (p) => [['Hours', str(p, 'hours')]],
  },
  'notion.body': {
    title: 'Update Notion page body', verb: 'Update',
    summary: (p) => [['Task', str(p, 'task_id')]],
  },
  'task.create': {
    title: 'Create task', verb: 'Create',
    summary: (p) => [['Title', str(p, 'title')]],
  },
  'task.add_repo': {
    title: 'Add repo to task', verb: 'Add repo',
    summary: (p) => [['Repo', str(p, 'project') || str(p, 'slug')]],
  },
  'task.create_from_explorer': {
    title: 'Promote explorer to task', verb: 'Create task',
    summary: (p) => [['Title', str(p, 'title')]],
  },
};

/** Presentation for an op, with a safe fallback for anything not mapped. */
export function opMeta(opType: string): OpMeta {
  return OPS[opType] ?? {
    title: opType,
    verb: 'Approve',
    summary: () => [],
  };
}
