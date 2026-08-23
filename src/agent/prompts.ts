// Canned agent asks, in one place.
//
// Every prompt NAMES ITS SESSION. Agents always run at the worktree root, never
// inside a task or repo directory, so cwd carries no task context — a prompt that
// says "this session" is ambiguous to the agent and will be answered about
// whatever it last looked at.
//
// These prompts say WHAT TO DO and nothing about how to write. Every rule for
// commit messages, MR text, annotations and task bodies lives in ONE place: the MCP
// tool descriptions (src-tauri/src/mcp_server/tools/definitions.rs), which the agent
// reads at the moment of the call. Repeating them here got them ignored in all three
// places, so do not add them back.

import type { SessionKind } from '../shared/store';

export interface PromptContext {
  /** Task / session short id — always interpolated into the prompt. */
  shortId: string;
  kind: SessionKind;
  /** First repo's project name, for prompts that mention the repo. */
  project?: string;
  /** MR number as `!42`, when the session has one. */
  mrNumber?: string;
  /** Which task source to file at. Only needed when more than one is set up. */
  provider?: string;
}

/** A button in the pill: a label plus the prompt it sends. */
export interface AgentAction {
  id: string;
  label: string;
  title: string;
  build: (ctx: PromptContext) => string;
}

const coReview = (ctx: PromptContext) =>
  `You're co-reviewing session ${ctx.shortId} (MR ${ctx.mrNumber ?? '(unknown)'} in ${ctx.project ?? 'the repo'}).\n\n` +
  '1. Read the whole change with get_task_diff, then tell me in the chat what this ' +
  'MR does and why. If needed give some context about the components it touches.\n' +
  '2. Then leave annotations (create_annotation) only where something is actually ' +
  'wrong or risky: bugs, broken edge cases, security problems, breaking changes. ' +
  'Skip style nits and anything you would phrase as "consider…".\n';

const createTask = (ctx: PromptContext) =>
  `File a task from our work in explorer session ${ctx.shortId}` +
  (ctx.provider ? ` in ${ctx.provider}` : '') +
  '. Call get_task_template first, then create_task_from_explorer with a one-line ' +
  "title and a body under that template's headings" +
  (ctx.provider ? ` and provider "${ctx.provider}"` : '') +
  '.\n';

// Chat output, not a tool field — so the shape guidance has to be here.
const summarize = (ctx: PromptContext) =>
  `In session ${ctx.shortId}: read the current change with get_task_diff and tell me ` +
  'in the chat what it does and why. Cover the change as a whole, no file-by-file ' +
  'walkthrough.\n';

const openMr = (ctx: PromptContext) =>
  `In session ${ctx.shortId}: open an MR for the work on this branch. Read ` +
  'get_task_diff and get_commit_log first so the description matches what actually ' +
  "changed, then call create_mr. I'll approve it before it goes out.\n";

const reviewFixes = (ctx: PromptContext) =>
  `In session ${ctx.shortId}: read the open annotations with get_annotations and fix ` +
  'the ones that are real problems, in this worktree. Skip any you disagree with and ' +
  "tell me why in one line. Don't resolve an annotation you didn't actually fix. " +
  'Answer questions.\n';

/** Actions offered per session kind — the pill shows only what makes sense. */
const ACTIONS: Record<SessionKind, AgentAction[]> = {
  review: [
    { id: 'co-review', label: 'co-review', title: 'Summarize the MR and annotate real problems', build: coReview },
    { id: 'fix-notes', label: 'fix notes', title: 'Fix the open annotations', build: reviewFixes },
  ],
  explorer: [
    { id: 'create-task', label: 'create task', title: 'Draft a task from this session', build: createTask },
    { id: 'summarize', label: 'summarize', title: 'Explain the current change', build: summarize },
  ],
  task: [
    { id: 'open-mr', label: 'open MR', title: 'Draft an MR for this branch (you approve it)', build: openMr },
    { id: 'fix-notes', label: 'fix notes', title: 'Fix the open annotations', build: reviewFixes },
  ],
};

export function actionsFor(kind: SessionKind): AgentAction[] {
  return ACTIONS[kind] ?? [];
}
